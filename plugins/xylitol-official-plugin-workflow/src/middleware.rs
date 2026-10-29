use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use xylitol_plugin_sdk::message::{AgentMessage, AgentPart, XyLlmMessage};
use xylitol_plugin_sdk::rt::Ctx;
use xylitol_plugin_sdk::session::{
    Workflow, WorkflowStep, WorkflowStepStatus, blocked_workflow_step, latest_workflow,
    runnable_workflow_step, workflow_downstream_ids,
};
use xylitol_plugin_sdk::{
    AfterToolCtx, BeforeLlmRequestCtx, DecideContinueCtx, LoopMiddleware, LoopStage, LoopStep,
    RuntimeSessionStore, XySessionStore,
};

use crate::config::WorkflowConfig;

const WORKFLOW_STAGES: &[LoopStage] = &[
    LoopStage::BeforeLlmRequest,
    LoopStage::AfterTool,
    LoopStage::DecideContinue,
];
const WORKFLOW_CONTRACT: &str = "Workflow: create a workflow before acting. Call workflow_write \
with the full workflow: {\"goal\":\"...\",\"steps\":[{\"id\":\"s1\",\"description\":\"...\",\
\"depends_on\":[],\"inputs\":[],\"outputs\":[],\"tools\":[\"read\",\"write\"],\"risk\":\"low\",\
\"verification\":\"...\",\"rollback\":\"...\",\"requires_human\":false,\"status\":\"pending\"}]}. \
Declare the tools each step needs so unrelated tool failures do not fail the step. Do not emit a \
<plan> block in assistant text. Keep exactly one step in_progress; mark it completed only after \
verification passes.";

fn resolve_resume_cursor(plan: &Workflow) -> Workflow {
    let Some(cursor) = plan.resume_cursor.as_deref() else {
        return plan.clone();
    };
    let mut current = plan.clone();
    for id in cursor.split('/').filter(|part| !part.is_empty()) {
        let Some(step) = current.step(id) else {
            return plan.clone();
        };
        match step.sub_workflow.as_deref() {
            Some(sub) => current = sub.clone(),
            None => break,
        }
    }
    current
}

fn assistant_has_tool_calls(message: &AgentMessage) -> bool {
    match message {
        AgentMessage::Llm(XyLlmMessage::AssistantMessage { content, .. }) => content
            .iter()
            .any(|part| matches!(part, AgentPart::ToolCall { .. })),
        _ => false,
    }
}

fn workflow_properties(state: &WorkflowState, step: &str, blocked: bool) -> Vec<(String, String)> {
    vec![
        ("loop.middleware.workflow.step".into(), step.to_string()),
        (
            "loop.middleware.workflow.depth".into(),
            (state.stack.len() + 1).to_string(),
        ),
        (
            "loop.middleware.workflow.replan".into(),
            state.replan_count.to_string(),
        ),
        (
            "loop.middleware.workflow.blocked".into(),
            blocked.to_string(),
        ),
    ]
}

#[derive(Debug, Default)]
struct WorkflowState {
    session_id: Option<String>,
    plan: Option<Workflow>,
    stack: Vec<(Workflow, String)>,
    approved: HashSet<String>,
    replan_count: u32,
    contract_injected: bool,
    injected_step: Option<String>,
    summary_injected: bool,
    summary_done: bool,
    pending_workflow_error: Option<String>,
}

impl WorkflowState {
    fn step_to_fail(&mut self, tool_name: &str) -> Option<&mut WorkflowStep> {
        let step = self
            .plan
            .as_mut()?
            .steps
            .iter_mut()
            .find(|step| step.status == WorkflowStepStatus::InProgress)?;
        if !step.tools.is_empty() && !step.tools.iter().any(|tool| tool == tool_name) {
            return None;
        }
        Some(step)
    }
}

pub struct WorkflowMiddleware {
    store: Option<Arc<dyn XySessionStore>>,
    ctx: Option<Ctx>,
    max_replans: u32,
    state: Mutex<WorkflowState>,
    loaded: AtomicBool,
}

impl WorkflowMiddleware {
    pub fn new(
        store: Option<Arc<dyn XySessionStore>>,
        ctx: Option<Ctx>,
        config: &WorkflowConfig,
    ) -> Self {
        Self {
            store,
            ctx,
            max_replans: config.max_replans.max(1),
            state: Mutex::new(WorkflowState::default()),
            loaded: AtomicBool::new(false),
        }
    }

    fn resolve_store(&self) -> Option<Arc<dyn XySessionStore>> {
        self.ctx
            .as_ref()
            .and_then(|ctx| ctx.get::<RuntimeSessionStore>())
            .map(|store| store.0.clone())
            .or_else(|| {
                self.ctx
                    .as_ref()
                    .and_then(|ctx| ctx.get::<dyn XySessionStore>())
            })
            .or_else(|| self.store.clone())
    }

    fn lock_state(&self) -> std::sync::MutexGuard<'_, WorkflowState> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    async fn load_workflow(&self, session_id: Option<&str>) {
        let (Some(store), Some(session_id)) = (self.resolve_store(), session_id) else {
            return;
        };
        let Ok(branch) = store.load_leaf_branch(session_id).await else {
            return;
        };
        let Some(plan) = latest_workflow(&branch) else {
            return;
        };
        let plan = resolve_resume_cursor(&plan);
        let mut state = self.lock_state();
        if state.plan.is_none() {
            state.plan = Some(plan);
            state.contract_injected = true;
        }
    }

    async fn reload_workflow(&self) {
        let session_id = self.lock_state().session_id.clone();
        let (Some(store), Some(session_id)) = (self.resolve_store(), session_id.as_deref()) else {
            return;
        };
        let Ok(branch) = store.load_leaf_branch(session_id).await else {
            return;
        };
        let Some(plan) = latest_workflow(&branch) else {
            return;
        };
        let plan = resolve_resume_cursor(&plan);
        let had_failed = self.lock_state().plan.as_ref().is_some_and(|previous| {
            previous
                .steps
                .iter()
                .any(|step| step.status == WorkflowStepStatus::Failed)
        });
        let mut state = self.lock_state();
        state.stack.clear();
        state.plan = Some(plan);
        state.injected_step = None;
        state.summary_injected = false;
        state.summary_done = false;
        state.contract_injected = true;
        state.pending_workflow_error = None;
        if had_failed {
            state.replan_count += 1;
        }
    }

    fn step_instruction(step: &WorkflowStep, plan: &Workflow) -> String {
        let mut lines = vec![
            "<plan_step>".to_string(),
            format!(
                "Workflow {}/{}: current step {} — {}.",
                plan.completed_count(),
                plan.steps.len(),
                step.id,
                step.description
            ),
        ];
        if !step.inputs.is_empty() {
            lines.push(format!("Inputs: {}.", step.inputs.join(", ")));
        }
        if let Some(verification) = &step.verification {
            lines.push(format!("Verification: {verification}."));
        }
        lines.push("Do not start dependent steps before this one completes.".into());
        lines.push("</plan_step>".into());
        lines.join("\n")
    }

    fn replan_instruction(failed: &WorkflowStep, plan: &Workflow) -> String {
        let downstream = workflow_downstream_ids(plan, &failed.id);
        let downstream_list = if downstream.is_empty() {
            "none".to_string()
        } else {
            let mut ids: Vec<String> = downstream.into_iter().collect();
            ids.sort();
            ids.join(", ")
        };
        format!(
            "<workflow_replan>\nStep {} failed: {}.\nRewrite only {} and its downstream steps ({downstream_list}). \
Keep completed steps unchanged. Call workflow_write with the full workflow.\n</workflow_replan>",
            failed.id,
            failed
                .output_ref
                .as_deref()
                .unwrap_or("verification did not pass"),
            failed.id
        )
    }

    fn blocked_instruction(step: &WorkflowStep) -> String {
        format!(
            "<plan_blocked>\nStep {} — {} requires human approval. Stop and ask the user; \
the next user message approves it once.\n</plan_blocked>",
            step.id, step.description
        )
    }
}

#[async_trait]
impl LoopMiddleware for WorkflowMiddleware {
    fn id(&self) -> &str {
        "workflow"
    }

    fn order(&self) -> i64 {
        190
    }

    fn stages(&self) -> &'static [LoopStage] {
        WORKFLOW_STAGES
    }

    async fn before_llm_request(
        &self,
        mut ctx: BeforeLlmRequestCtx,
    ) -> LoopStep<BeforeLlmRequestCtx> {
        if !self.loaded.swap(true, Ordering::SeqCst) {
            self.load_workflow(ctx.session_id.as_deref()).await;
        }
        let mut state = self.lock_state();
        state.session_id = ctx.session_id.clone();
        if let Some(error) = state.pending_workflow_error.take() {
            ctx.messages
                .push(AgentMessage::Llm(XyLlmMessage::user(format!(
                    "<workflow_error>\n{error}\n</workflow_error>"
                ))));
        }
        if state.plan.is_none() {
            if !state.contract_injected {
                ctx.messages
                    .push(AgentMessage::Llm(XyLlmMessage::user(WORKFLOW_CONTRACT)));
                state.contract_injected = true;
            }
            return LoopStep::continued(ctx);
        }
        loop {
            let plan = state.plan.clone().expect("workflow plan");
            if let Some(blocked) = blocked_workflow_step(&plan, &state.approved) {
                if state.injected_step.as_deref() != Some(&blocked.id) {
                    ctx.messages.push(AgentMessage::Llm(XyLlmMessage::user(
                        Self::blocked_instruction(blocked),
                    )));
                    state.injected_step = Some(blocked.id.clone());
                }
                return LoopStep::continued(ctx).with_properties(workflow_properties(
                    &state,
                    &blocked.id,
                    true,
                ));
            }
            if let Some(failed) = plan
                .steps
                .iter()
                .find(|step| step.status == WorkflowStepStatus::Failed)
            {
                if state.injected_step.as_deref() != Some(&failed.id) {
                    ctx.messages.push(AgentMessage::Llm(XyLlmMessage::user(
                        Self::replan_instruction(failed, &plan),
                    )));
                    state.injected_step = Some(failed.id.clone());
                }
                return LoopStep::continued(ctx)
                    .with_properties(workflow_properties(&state, &failed.id, false));
            }
            if plan.all_completed() {
                if let Some((parent, container_id)) = state.stack.pop() {
                    let mut parent = parent;
                    if let Some(container) =
                        parent.steps.iter_mut().find(|step| step.id == container_id)
                    {
                        container.status = WorkflowStepStatus::Completed;
                    }
                    state.plan = Some(parent);
                    state.injected_step = None;
                    state.summary_injected = false;
                    state.summary_done = false;
                    continue;
                }
                if !state.summary_injected {
                    ctx.messages.push(AgentMessage::Llm(XyLlmMessage::user(
                        "<plan_complete>\nAll workflow steps are completed. Summarize what changed and how it was verified.\n</plan_complete>",
                    )));
                    state.summary_injected = true;
                }
                return LoopStep::continued(ctx)
                    .with_property("loop.middleware.workflow.completed", "true");
            }
            let Some(step) = runnable_workflow_step(&plan, &state.approved) else {
                return LoopStep::continued(ctx);
            };
            if let Some(sub) = step.sub_workflow.as_deref() {
                let container_id = step.id.clone();
                let mut parent = plan.clone();
                if let Some(container) = parent
                    .steps
                    .iter_mut()
                    .find(|candidate| candidate.id == container_id)
                {
                    container.status = WorkflowStepStatus::InProgress;
                }
                let child = sub.clone();
                state.stack.push((parent, container_id));
                state.plan = Some(child);
                state.injected_step = None;
                state.summary_injected = false;
                state.summary_done = false;
                state.contract_injected = true;
                continue;
            }
            if state.injected_step.as_deref() != Some(&step.id) {
                ctx.messages.push(AgentMessage::Llm(XyLlmMessage::user(
                    Self::step_instruction(step, &plan),
                )));
                state.injected_step = Some(step.id.clone());
            }
            return LoopStep::continued(ctx)
                .with_properties(workflow_properties(&state, &step.id, false));
        }
    }

    async fn after_tool(&self, ctx: AfterToolCtx) -> LoopStep<AfterToolCtx> {
        if matches!(ctx.tool_name.as_str(), "workflow_write" | "workflow_fail") {
            if ctx.is_error {
                let fallback = format!("{} failed", ctx.tool_name);
                let text = ctx
                    .parts
                    .iter()
                    .find_map(|part| match part {
                        AgentPart::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .unwrap_or(fallback.as_str());
                self.lock_state().pending_workflow_error = Some(text.to_string());
            } else {
                self.reload_workflow().await;
            }
            return LoopStep::continued(ctx);
        }
        if ctx.is_error {
            let mut state = self.lock_state();
            let failed_id = {
                let Some(step) = state.step_to_fail(&ctx.tool_name) else {
                    return LoopStep::continued(ctx);
                };
                step.status = WorkflowStepStatus::Failed;
                step.output_ref = Some(format!("failure:{}", ctx.tool_call_id));
                step.id.clone()
            };
            state.approved.remove(&failed_id);
            state.injected_step = None;
        }
        LoopStep::continued(ctx)
    }

    async fn decide_continue(&self, ctx: DecideContinueCtx) -> LoopStep<DecideContinueCtx> {
        let mut state = self.lock_state();
        let Some(plan) = state.plan.clone() else {
            if ctx
                .assistant
                .as_ref()
                .is_some_and(|assistant| !assistant_has_tool_calls(assistant))
            {
                return LoopStep::stopped(ctx);
            }
            return LoopStep::continued(ctx);
        };
        if let Some(blocked) = blocked_workflow_step(&plan, &state.approved) {
            if ctx.has_new_user_messages {
                state.approved.insert(blocked.id.clone());
                state.injected_step = None;
                return LoopStep::continued(ctx);
            }
            return LoopStep::stopped(ctx);
        }
        if plan
            .steps
            .iter()
            .any(|step| step.status == WorkflowStepStatus::Failed)
        {
            if state.replan_count >= self.max_replans {
                log::warn!(
                    target: "xylitol::loop-policies",
                    "workflow stopped after {} replan(s)",
                    state.replan_count
                );
                if let Some((parent, container_id)) = state.stack.pop() {
                    let mut parent = parent;
                    if let Some(container) =
                        parent.steps.iter_mut().find(|step| step.id == container_id)
                    {
                        container.status = WorkflowStepStatus::Failed;
                    }
                    state.plan = Some(parent);
                    state.injected_step = None;
                    return LoopStep::continued(ctx);
                }
                return LoopStep::stopped(ctx);
            }
            return LoopStep::continued(ctx);
        }
        if plan.all_completed() {
            if !state.stack.is_empty() {
                return LoopStep::continued(ctx);
            }
            if state.summary_done {
                return LoopStep::stopped(ctx);
            }
            if state.summary_injected
                && ctx
                    .assistant
                    .as_ref()
                    .is_some_and(|assistant| !assistant_has_tool_calls(assistant))
            {
                state.summary_done = true;
                return LoopStep::stopped(ctx);
            }
            return LoopStep::continued(ctx);
        }
        if runnable_workflow_step(&plan, &state.approved).is_some() {
            LoopStep::continued(ctx)
        } else {
            LoopStep::stopped(ctx)
        }
    }
}

pub fn workflow_middleware(
    store: Option<Arc<dyn XySessionStore>>,
    ctx: Option<Ctx>,
    config: &WorkflowConfig,
) -> Arc<dyn LoopMiddleware> {
    Arc::new(WorkflowMiddleware::new(store, ctx, config))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use xylitol_plugin_sdk::LoopControl;
    use xylitol_plugin_sdk::session::{Workflow, WorkflowRisk, WorkflowStep, WorkflowStepStatus};

    fn model_ctx(session_id: Option<&str>) -> BeforeLlmRequestCtx {
        BeforeLlmRequestCtx {
            turn_index: 0,
            messages: Vec::new(),
            tool_schemas: Vec::new(),
            options: xylitol_plugin_sdk::XyGenerateOptions::default(),
            todo: None,
            plan_mode: None,
            session_id: session_id.map(str::to_string),
        }
    }

    fn tool_ctx(name: &str, text: &str, is_error: bool) -> AfterToolCtx {
        AfterToolCtx {
            turn_index: 0,
            tool_call_id: "call-1".into(),
            tool_name: name.into(),
            parts: vec![AgentPart::text(text)],
            details: None,
            is_error,
        }
    }

    fn workflow() -> Workflow {
        Workflow {
            goal: "ship".into(),
            steps: vec![WorkflowStep {
                id: "s1".into(),
                description: "do it".into(),
                depends_on: Vec::new(),
                inputs: Vec::new(),
                outputs: Vec::new(),
                tools: Vec::new(),
                risk: WorkflowRisk::Low,
                verification: Some("tests pass".into()),
                rollback: None,
                requires_human: false,
                status: WorkflowStepStatus::Pending,
                output_ref: None,
                sub_workflow: None,
            }],
            resume_cursor: None,
            updated_at: 1,
        }
    }

    #[tokio::test]
    async fn contract_points_at_workflow_write_without_raw_block() {
        let mw = workflow_middleware(None, None, &WorkflowConfig::default());
        let out = mw.before_llm_request(model_ctx(None)).await;
        let text = out
            .ctx
            .messages
            .iter()
            .map(|message| message.text())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("workflow_write"), "{text}");
        assert!(!text.contains("<plan>{json}</plan>"), "{text}");
    }

    #[tokio::test]
    async fn workflow_write_success_reloads_and_injects_step() {
        let store: Arc<dyn XySessionStore> =
            Arc::new(xylitol_official_plugin_session::SessionManager::in_memory());
        store.create("s1", Some("."), None).await.expect("create");
        store
            .append_session_entry("s1", &workflow().to_custom_entry_shell())
            .await
            .expect("seed");
        let mw = workflow_middleware(Some(store), None, &WorkflowConfig::default());
        mw.before_llm_request(model_ctx(Some("s1"))).await;
        mw.after_tool(tool_ctx("workflow_write", "{}", false)).await;
        let out = mw.before_llm_request(model_ctx(Some("s1"))).await;
        let text = out
            .ctx
            .messages
            .iter()
            .map(|message| message.text())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("<plan_step>"), "{text}");
        assert!(text.contains("s1"), "{text}");
    }

    #[tokio::test]
    async fn workflow_write_error_is_injected_once() {
        let mw = workflow_middleware(None, None, &WorkflowConfig::default());
        mw.before_llm_request(model_ctx(None)).await;
        mw.after_tool(tool_ctx(
            "workflow_write",
            "duplicate workflow step id: s1",
            true,
        ))
        .await;
        let out = mw.before_llm_request(model_ctx(None)).await;
        let text = out
            .ctx
            .messages
            .iter()
            .map(|message| message.text())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("<workflow_error>"), "{text}");
        assert!(text.contains("duplicate workflow step id"), "{text}");
        let out = mw.before_llm_request(model_ctx(None)).await;
        assert!(
            !out.ctx
                .messages
                .iter()
                .any(|message| message.text().contains("<workflow_error>")),
            "workflow error must be injected once"
        );
    }

    fn assistant_with(parts: Vec<AgentPart>) -> AgentMessage {
        AgentMessage::Llm(XyLlmMessage::AssistantMessage {
            content: parts,
            stop_reason: None,
            usage: None,
            api: String::new(),
            provider: String::new(),
            model: String::new(),
            response_id: None,
            error_message: None,
            timestamp: 0,
            diagnostics: Vec::new(),
        })
    }

    fn decide_ctx_with(assistant: Option<AgentMessage>) -> DecideContinueCtx {
        DecideContinueCtx {
            turn_index: 0,
            settle_count: 0,
            has_new_user_messages: false,
            assistant,
            history_len: 2,
            compact: None,
        }
    }

    #[tokio::test]
    async fn no_workflow_stops_after_final_answer() {
        let mw = workflow_middleware(None, None, &WorkflowConfig::default());
        let control = mw
            .decide_continue(decide_ctx_with(Some(assistant_with(vec![
                AgentPart::text("done"),
            ]))))
            .await
            .control;
        assert_eq!(
            control,
            LoopControl::Stop,
            "a final answer without a workflow must stop instead of forcing more turns"
        );
    }

    #[tokio::test]
    async fn no_workflow_continues_while_tools_run() {
        let mw = workflow_middleware(None, None, &WorkflowConfig::default());
        let control = mw
            .decide_continue(decide_ctx_with(Some(assistant_with(vec![
                AgentPart::ToolCall {
                    id: "call-1".into(),
                    name: "read".into(),
                    arguments: serde_json::json!({ "path": "src/lib.rs" }),
                },
            ]))))
            .await
            .control;
        assert_eq!(control, LoopControl::Continue);
    }

    #[test]
    fn identity_and_stages_are_stable() {
        let mw = WorkflowMiddleware::new(None, None, &WorkflowConfig::default());
        assert_eq!(mw.id(), "workflow");
        assert_eq!(mw.order(), 190);
        assert_eq!(mw.stages(), WORKFLOW_STAGES);
    }
}

#[cfg(test)]
mod nested_tests {
    use std::sync::Arc;

    use super::*;
    use xylitol_plugin_sdk::LoopControl;
    use xylitol_plugin_sdk::session::{Workflow, WorkflowRisk, WorkflowStep, WorkflowStepStatus};

    fn leaf(id: &str, status: WorkflowStepStatus) -> WorkflowStep {
        WorkflowStep {
            id: id.into(),
            description: format!("do {id}"),
            depends_on: Vec::new(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            tools: Vec::new(),
            risk: WorkflowRisk::Low,
            verification: None,
            rollback: None,
            requires_human: false,
            status,
            output_ref: None,
            sub_workflow: None,
        }
    }

    fn container(id: &str, child: Workflow, status: WorkflowStepStatus) -> WorkflowStep {
        WorkflowStep {
            sub_workflow: Some(Box::new(child)),
            ..leaf(id, status)
        }
    }

    fn workflow(steps: Vec<WorkflowStep>, resume_cursor: Option<String>) -> Workflow {
        Workflow {
            goal: "ship".into(),
            steps,
            resume_cursor,
            updated_at: 1,
        }
    }

    fn model_ctx(session_id: &str) -> BeforeLlmRequestCtx {
        BeforeLlmRequestCtx {
            turn_index: 0,
            messages: Vec::new(),
            tool_schemas: Vec::new(),
            options: xylitol_plugin_sdk::XyGenerateOptions::default(),
            todo: None,
            plan_mode: None,
            session_id: Some(session_id.into()),
        }
    }

    fn decide_ctx() -> DecideContinueCtx {
        DecideContinueCtx {
            turn_index: 0,
            settle_count: 0,
            has_new_user_messages: false,
            assistant: None,
            history_len: 2,
            compact: None,
        }
    }

    async fn seeded(workflow: Workflow) -> (Arc<dyn XySessionStore>, Arc<dyn LoopMiddleware>) {
        seeded_with(workflow, WorkflowConfig::default()).await
    }

    async fn seeded_with(
        workflow: Workflow,
        config: WorkflowConfig,
    ) -> (Arc<dyn XySessionStore>, Arc<dyn LoopMiddleware>) {
        let store: Arc<dyn XySessionStore> =
            Arc::new(xylitol_official_plugin_session::SessionManager::in_memory());
        store.create("s1", Some("."), None).await.expect("create");
        store
            .append_session_entry("s1", &workflow.to_custom_entry_shell())
            .await
            .expect("seed");
        let mw = workflow_middleware(Some(store.clone()), None, &config);
        (store, mw)
    }

    fn text(out: &LoopStep<BeforeLlmRequestCtx>) -> String {
        out.ctx
            .messages
            .iter()
            .map(|message| message.text())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[tokio::test]
    async fn container_expands_to_child_workflow() {
        let child = workflow(vec![leaf("c1", WorkflowStepStatus::Pending)], None);
        let root = workflow(
            vec![container("s1", child, WorkflowStepStatus::Pending)],
            None,
        );
        let (_store, mw) = seeded(root).await;
        let out = mw.before_llm_request(model_ctx("s1")).await;
        let text = text(&out);
        assert!(text.contains("<plan_step>"), "{text}");
        assert!(text.contains("c1"), "{text}");
    }

    #[tokio::test]
    async fn completed_child_marks_container_completed() {
        let child = workflow(vec![leaf("c1", WorkflowStepStatus::Completed)], None);
        let root = workflow(
            vec![container("s1", child, WorkflowStepStatus::InProgress)],
            None,
        );
        let (store, mw) = seeded(root.clone()).await;
        mw.before_llm_request(model_ctx("s1")).await;
        let mut completed_root = root;
        completed_root.steps[0].status = WorkflowStepStatus::Completed;
        store
            .append_session_entry("s1", &completed_root.to_custom_entry_shell())
            .await
            .expect("append completed");
        mw.after_tool(AfterToolCtx {
            turn_index: 0,
            tool_call_id: "c1".into(),
            tool_name: "workflow_write".into(),
            parts: vec![AgentPart::text("{}")],
            details: None,
            is_error: false,
        })
        .await;
        let out = mw.before_llm_request(model_ctx("s1")).await;
        let text = text(&out);
        assert!(text.contains("<plan_complete>"), "{text}");
    }

    #[tokio::test]
    async fn resume_cursor_enters_nested_workflow() {
        let child = workflow(vec![leaf("c1", WorkflowStepStatus::Pending)], None);
        let root = workflow(
            vec![container("s1", child, WorkflowStepStatus::InProgress)],
            Some("s1".into()),
        );
        let (_store, mw) = seeded(root).await;
        let out = mw.before_llm_request(model_ctx("s1")).await;
        let text = text(&out);
        assert!(text.contains("c1"), "{text}");
        assert!(!text.contains("s1"), "root container must not be injected");
    }

    fn tool_ctx(name: &str, text: &str, is_error: bool) -> AfterToolCtx {
        AfterToolCtx {
            turn_index: 0,
            tool_call_id: "call-1".into(),
            tool_name: name.into(),
            parts: vec![AgentPart::text(text)],
            details: None,
            is_error,
        }
    }

    async fn seeded_concrete(workflow: Workflow) -> (Arc<dyn XySessionStore>, WorkflowMiddleware) {
        let store: Arc<dyn XySessionStore> =
            Arc::new(xylitol_official_plugin_session::SessionManager::in_memory());
        store.create("s1", Some("."), None).await.expect("create");
        store
            .append_session_entry("s1", &workflow.to_custom_entry_shell())
            .await
            .expect("seed");
        let mw = WorkflowMiddleware::new(Some(store.clone()), None, &WorkflowConfig::default());
        (store, mw)
    }

    #[tokio::test]
    async fn declared_tools_ignore_unrelated_failures() {
        let mut wf = workflow(vec![leaf("s1", WorkflowStepStatus::InProgress)], None);
        wf.steps[0].tools = vec!["write".into()];
        let (_store, mw) = seeded_concrete(wf).await;
        mw.before_llm_request(model_ctx("s1")).await;
        mw.after_tool(tool_ctx("read", "missing file", true)).await;
        let state = mw.lock_state();
        assert_eq!(
            state.plan.as_ref().expect("plan").steps[0].status,
            WorkflowStepStatus::InProgress,
            "a failure outside the declared tools must not fail the step"
        );
    }

    #[tokio::test]
    async fn declared_tools_mark_matching_failure() {
        let mut wf = workflow(vec![leaf("s1", WorkflowStepStatus::InProgress)], None);
        wf.steps[0].tools = vec!["write".into()];
        let (_store, mw) = seeded_concrete(wf).await;
        mw.before_llm_request(model_ctx("s1")).await;
        mw.after_tool(tool_ctx("write", "permission denied", true))
            .await;
        let state = mw.lock_state();
        assert_eq!(
            state.plan.as_ref().expect("plan").steps[0].status,
            WorkflowStepStatus::Failed
        );
    }

    #[tokio::test]
    async fn legacy_workflow_without_tools_keeps_coarse_attribution() {
        let wf = workflow(vec![leaf("s1", WorkflowStepStatus::InProgress)], None);
        let (_store, mw) = seeded_concrete(wf).await;
        mw.before_llm_request(model_ctx("s1")).await;
        mw.after_tool(tool_ctx("read", "missing file", true)).await;
        let state = mw.lock_state();
        assert_eq!(
            state.plan.as_ref().expect("plan").steps[0].status,
            WorkflowStepStatus::Failed,
            "legacy workflows without declared tools keep the coarse attribution"
        );
    }

    #[tokio::test]
    async fn child_failure_bubbles_to_parent_after_replan_cap() {
        let child = workflow(vec![leaf("c1", WorkflowStepStatus::Failed)], None);
        let root = workflow(
            vec![container("s1", child, WorkflowStepStatus::Pending)],
            None,
        );
        let config = WorkflowConfig {
            max_replans: 1,
            ..Default::default()
        };
        let (_store, mw) = seeded_with(root, config).await;
        mw.before_llm_request(model_ctx("s1")).await;
        mw.after_tool(AfterToolCtx {
            turn_index: 0,
            tool_call_id: "c1".into(),
            tool_name: "workflow_write".into(),
            parts: vec![AgentPart::text("{}")],
            details: None,
            is_error: false,
        })
        .await;
        mw.before_llm_request(model_ctx("s1")).await;
        let control = mw.decide_continue(decide_ctx()).await.control;
        assert_eq!(control, LoopControl::Continue);
        let out = mw.before_llm_request(model_ctx("s1")).await;
        let text = text(&out);
        assert!(text.contains("<workflow_replan>"), "{text}");
        assert!(text.contains("s1"), "{text}");
    }
}
