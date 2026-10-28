use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use xylitol_plugin_sdk::message::{AgentMessage, AgentPart, XyLlmMessage};
use xylitol_plugin_sdk::rt::Ctx;
use xylitol_plugin_sdk::session::{
    Workflow, WorkflowStep, WorkflowStepStatus, blocked_workflow_step, latest_workflow,
    runnable_workflow_step, validate_workflow, validate_workflow_replan, workflow_downstream_ids,
};
use xylitol_plugin_sdk::{
    AfterModelCtx, AfterToolCtx, BeforeModelCtx, DecideContinueCtx, LoopMiddleware, LoopStage,
    LoopStep, RuntimeSessionStore, XySessionStore,
};

use crate::config::WorkflowConfig;

const WORKFLOW_STAGES: &[LoopStage] = &[
    LoopStage::BeforeModel,
    LoopStage::AfterModel,
    LoopStage::AfterTool,
    LoopStage::DecideContinue,
];
const WORKFLOW_CONTRACT: &str = "Workflow: create a workflow before acting. Output exactly one \
<plan>{json}</plan> block with the full workflow: {\"goal\":\"...\",\"steps\":[{\"id\":\"s1\",\
\"description\":\"...\",\"depends_on\":[],\"inputs\":[],\"outputs\":[],\"risk\":\"low\",\
\"verification\":\"...\",\"rollback\":\"...\",\"requires_human\":false,\"status\":\"pending\"}]}. \
Keep exactly one step in_progress; mark it completed only after verification passes.";

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
            "<plan_replan>\nStep {} failed: {}.\nRewrite only {} and its downstream steps ({downstream_list}). \
Keep completed steps unchanged. Output a full <plan>{{json}}</plan> block.\n</plan_replan>",
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

fn extract_workflow_json(text: &str) -> Option<&str> {
    let start = text.rfind("<plan>")? + "<plan>".len();
    let rest = &text[start..];
    let end = rest.find("</plan>")?;
    let json = rest[..end].trim();
    if json.is_empty() { None } else { Some(json) }
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
        if let Some(error) = state.pending_workflow_error.take() {
            ctx.messages
                .push(AgentMessage::Llm(XyLlmMessage::user(format!(
                    "<plan_error>\n{error}\n</plan_error>"
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

    async fn after_model(&self, ctx: AfterModelCtx) -> LoopStep<AfterModelCtx> {
        let assistant_text = ctx.assistant.text();
        let Some(json) = extract_workflow_json(&assistant_text) else {
            return LoopStep::continued(ctx);
        };
        let next: Workflow = match serde_json::from_str(json) {
            Ok(plan) => plan,
            Err(err) => {
                log::warn!(
                    target: "xylitol::loop-example",
                    "workflow block JSON error: {err}"
                );
                self.lock_state().pending_workflow_error =
                    Some(format!("workflow block is not valid JSON: {err}"));
                return LoopStep::continued(ctx);
            }
        };
        if let Err(err) = validate_workflow(&next) {
            log::warn!(
                target: "xylitol::loop-example",
                "workflow DAG invalid: {}",
                err.0
            );
            self.lock_state().pending_workflow_error = Some(err.0);
            return LoopStep::continued(ctx);
        }
        let previous = self.lock_state().plan.clone();
        if let Err(err) = validate_workflow_replan(previous.as_ref(), &next) {
            log::warn!(
                target: "xylitol::loop-example",
                "workflow replan invalid: {}",
                err.0
            );
            self.lock_state().pending_workflow_error = Some(err.0);
            return LoopStep::continued(ctx);
        }
        if let (Some(store), Some(session_id)) = (self.resolve_store(), ctx.session_id.as_deref())
            && let Err(err) = store
                .append_session_entry(session_id, &next.to_custom_entry_shell())
                .await
        {
            log::warn!(
                target: "xylitol::loop-example",
                "workflow persist failed: {err}"
            );
        }
        let had_failed = previous.as_ref().is_some_and(|plan| {
            plan.steps
                .iter()
                .any(|step| step.status == WorkflowStepStatus::Failed)
        });
        let mut state = self.lock_state();
        state.plan = Some(next);
        state.injected_step = None;
        state.summary_injected = false;
        state.summary_done = false;
        state.contract_injected = true;
        state.pending_workflow_error = None;
        if had_failed {
            state.replan_count += 1;
        }
        let plan = state.plan.clone().unwrap_or_default();
        state.approved.retain(|id| {
            plan.step(id).is_some_and(|step| {
                matches!(
                    step.status,
                    WorkflowStepStatus::Pending | WorkflowStepStatus::InProgress
                )
            })
        });
        LoopStep::continued(ctx)
    }

    async fn after_tool(&self, ctx: AfterToolCtx) -> LoopStep<AfterToolCtx> {
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
