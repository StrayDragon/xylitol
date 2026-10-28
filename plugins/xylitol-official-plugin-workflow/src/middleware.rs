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
    AfterToolCtx, BeforeModelCtx, DecideContinueCtx, LoopMiddleware, LoopStage, LoopStep,
    RuntimeSessionStore, XySessionStore,
};

use crate::config::WorkflowConfig;

const WORKFLOW_STAGES: &[LoopStage] = &[
    LoopStage::BeforeModel,
    LoopStage::AfterTool,
    LoopStage::DecideContinue,
];
const WORKFLOW_CONTRACT: &str = "Workflow: create a workflow before acting. Call workflow_write \
with the full workflow: {\"goal\":\"...\",\"steps\":[{\"id\":\"s1\",\"description\":\"...\",\
\"depends_on\":[],\"inputs\":[],\"outputs\":[],\"risk\":\"low\",\"verification\":\"...\",\
\"rollback\":\"...\",\"requires_human\":false,\"status\":\"pending\"}]}. Do not emit a <plan> \
block in assistant text. Keep exactly one step in_progress; mark it completed only after verification passes.";

fn assistant_has_tool_calls(message: &AgentMessage) -> bool {
    match message {
        AgentMessage::Llm(XyLlmMessage::AssistantMessage { content, .. }) => content
            .iter()
            .any(|part| matches!(part, AgentPart::ToolCall { .. })),
        _ => false,
    }
}

#[derive(Debug, Default)]
struct WorkflowState {
    session_id: Option<String>,
    plan: Option<Workflow>,
    approved: HashSet<String>,
    replan_count: u32,
    contract_injected: bool,
    injected_step: Option<String>,
    summary_injected: bool,
    summary_done: bool,
    pending_workflow_error: Option<String>,
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
        let had_failed = self.lock_state().plan.as_ref().is_some_and(|previous| {
            previous
                .steps
                .iter()
                .any(|step| step.status == WorkflowStepStatus::Failed)
        });
        let mut state = self.lock_state();
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

    async fn before_model(&self, mut ctx: BeforeModelCtx) -> LoopStep<BeforeModelCtx> {
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
        let Some(plan) = state.plan.clone() else {
            if !state.contract_injected {
                ctx.messages
                    .push(AgentMessage::Llm(XyLlmMessage::user(WORKFLOW_CONTRACT)));
                state.contract_injected = true;
            }
            return LoopStep::continued(ctx);
        };
        if let Some(blocked) = blocked_workflow_step(&plan, &state.approved) {
            if state.injected_step.as_deref() != Some(&blocked.id) {
                ctx.messages.push(AgentMessage::Llm(XyLlmMessage::user(
                    Self::blocked_instruction(blocked),
                )));
                state.injected_step = Some(blocked.id.clone());
            }
            return LoopStep::continued(ctx);
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
            return LoopStep::continued(ctx);
        }
        if plan.all_completed() {
            if !state.summary_injected {
                ctx.messages.push(AgentMessage::Llm(XyLlmMessage::user(
                    "<plan_complete>\nAll workflow steps are completed. Summarize what changed and how it was verified.\n</plan_complete>",
                )));
                state.summary_injected = true;
            }
            return LoopStep::continued(ctx);
        }
        if let Some(step) = runnable_workflow_step(&plan, &state.approved)
            && state.injected_step.as_deref() != Some(&step.id)
        {
            ctx.messages.push(AgentMessage::Llm(XyLlmMessage::user(
                Self::step_instruction(step, &plan),
            )));
            state.injected_step = Some(step.id.clone());
        }
        LoopStep::continued(ctx)
    }

    async fn after_tool(&self, ctx: AfterToolCtx) -> LoopStep<AfterToolCtx> {
        if ctx.tool_name == "workflow_write" {
            if ctx.is_error {
                let text = ctx
                    .parts
                    .iter()
                    .find_map(|part| match part {
                        AgentPart::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .unwrap_or("workflow_write failed");
                self.lock_state().pending_workflow_error = Some(text.to_string());
            } else {
                self.reload_workflow().await;
            }
            return LoopStep::continued(ctx);
        }
        if ctx.is_error {
            let mut state = self.lock_state();
            if let Some(plan) = &mut state.plan
                && let Some(step) = plan
                    .steps
                    .iter_mut()
                    .find(|step| step.status == WorkflowStepStatus::InProgress)
            {
                step.status = WorkflowStepStatus::Failed;
                step.output_ref = Some(format!("failure:{}", ctx.tool_call_id));
                let id = step.id.clone();
                state.approved.remove(&id);
                state.injected_step = None;
            }
        }
        LoopStep::continued(ctx)
    }

    async fn decide_continue(&self, ctx: DecideContinueCtx) -> LoopStep<DecideContinueCtx> {
        let mut state = self.lock_state();
        let Some(plan) = state.plan.clone() else {
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
                    target: "xylitol::loop-example",
                    "workflow stopped after {} replan(s)",
                    state.replan_count
                );
                return LoopStep::stopped(ctx);
            }
            return LoopStep::continued(ctx);
        }
        if plan.all_completed() {
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
    use xylitol_plugin_sdk::session::{Workflow, WorkflowRisk, WorkflowStep, WorkflowStepStatus};

    fn model_ctx(session_id: Option<&str>) -> BeforeModelCtx {
        BeforeModelCtx {
            turn_index: 0,
            messages: Vec::new(),
            tool_schemas: Vec::new(),
            options: xylitol_plugin_sdk::XyGenerateOptions::default(),
            todo: None,
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
                risk: WorkflowRisk::Low,
                verification: Some("tests pass".into()),
                rollback: None,
                requires_human: false,
                status: WorkflowStepStatus::Pending,
                output_ref: None,
            }],
            resume_cursor: None,
            updated_at: 1,
        }
    }

    #[tokio::test]
    async fn contract_points_at_workflow_write_without_raw_block() {
        let mw = workflow_middleware(None, None, &WorkflowConfig::default());
        let out = mw.before_model(model_ctx(None)).await;
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
        mw.before_model(model_ctx(Some("s1"))).await;
        mw.after_tool(tool_ctx("workflow_write", "{}", false)).await;
        let out = mw.before_model(model_ctx(Some("s1"))).await;
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
        mw.before_model(model_ctx(None)).await;
        mw.after_tool(tool_ctx(
            "workflow_write",
            "duplicate workflow step id: s1",
            true,
        ))
        .await;
        let out = mw.before_model(model_ctx(None)).await;
        let text = out
            .ctx
            .messages
            .iter()
            .map(|message| message.text())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("<workflow_error>"), "{text}");
        assert!(text.contains("duplicate workflow step id"), "{text}");
        let out = mw.before_model(model_ctx(None)).await;
        assert!(
            !out.ctx
                .messages
                .iter()
                .any(|message| message.text().contains("<workflow_error>")),
            "workflow error must be injected once"
        );
    }

    #[test]
    fn identity_and_stages_are_stable() {
        let mw = WorkflowMiddleware::new(None, None, &WorkflowConfig::default());
        assert_eq!(mw.id(), "workflow");
        assert_eq!(mw.order(), 190);
        assert_eq!(mw.stages(), WORKFLOW_STAGES);
    }
}
