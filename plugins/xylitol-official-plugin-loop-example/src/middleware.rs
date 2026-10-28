use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::Value;

use xylitol_plugin_sdk::message::{AgentMessage, AgentPart, XyLlmMessage};
use xylitol_plugin_sdk::rt::Ctx;
use xylitol_plugin_sdk::session::{
    TodoCheckpoint, TodoCheckpointStep, TodoList, TodoResumeCursor, TodoStatus, Workflow,
    WorkflowStep, WorkflowStepStatus, blocked_workflow_step, latest_todo_checkpoint,
    latest_workflow, runnable_workflow_step, validate_workflow, validate_workflow_replan,
    workflow_downstream_ids,
};
use xylitol_plugin_sdk::{
    AfterModelCtx, AfterToolCtx, AfterTurnCtx, BeforeModelCtx, BeforeToolCtx, BeforeTurnCtx,
    DecideContinueCtx, LoopMiddleware, LoopStage, LoopStep, RuntimeSessionStore, XySessionStore,
};

use crate::config::{
    GeneratorCriticConfig, LoopExampleConfig, SelfHealConfig, ToolExposureConfig, WorkflowConfig,
};

const STAGES: &[LoopStage] = &[LoopStage::BeforeTool, LoopStage::DecideContinue];
pub const DEFAULT_TODO_PROGRESS_SETTLES: u32 = 8;
const TODO_PROGRESS_STAGES: &[LoopStage] = &[
    LoopStage::BeforeTurn,
    LoopStage::AfterTool,
    LoopStage::DecideContinue,
    LoopStage::AfterTurn,
];
const PROGRESS_STAGES: &[LoopStage] = &[LoopStage::BeforeModel, LoopStage::AfterTool];
const TOOL_EXPOSURE_STAGES: &[LoopStage] = &[LoopStage::BeforeModel];
const CHECKPOINT_STAGES: &[LoopStage] = &[
    LoopStage::BeforeModel,
    LoopStage::AfterTool,
    LoopStage::AfterTurn,
];
const SELF_HEAL_STAGES: &[LoopStage] = &[LoopStage::AfterTool, LoopStage::DecideContinue];
const CRITIC_STAGES: &[LoopStage] = &[
    LoopStage::BeforeModel,
    LoopStage::AfterModel,
    LoopStage::DecideContinue,
];
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
const DEFAULT_CRITIC_RUBRIC: &[&str] = &[
    "correctness: the answer matches what the tool results and code actually show",
    "evidence: cite concrete tool results, file paths or commands",
    "boundaries: call out unverified assumptions and missing checks",
    "verifiability: state how the result can be verified",
];
const DEFAULT_SELF_HEAL_SIGNATURES: &[&str] = &[
    "test failed",
    "tests failed",
    "assertion failed",
    "error[e",
    "compilation failed",
    "lint failed",
    "panicked at",
];
const TODO_PROGRESS_CONTRACT: &str = "Workflow mode: before acting, write the checklist with todo_rewrite. \
Keep exactly one item in_progress and mark finished items with todo_update. \
Do not claim completion while items remain pending.";

pub struct ExampleLoopMiddleware {
    config: LoopExampleConfig,
}

impl ExampleLoopMiddleware {
    pub fn new(config: LoopExampleConfig) -> Self {
        Self { config }
    }

    fn denies(&self, command: &str) -> bool {
        self.config
            .deny_bash_patterns
            .iter()
            .any(|pattern| command.contains(pattern.as_str()))
    }
}

#[async_trait]
impl LoopMiddleware for ExampleLoopMiddleware {
    fn id(&self) -> &str {
        "loop-example"
    }

    fn order(&self) -> i64 {
        100
    }

    fn stages(&self) -> &'static [LoopStage] {
        STAGES
    }

    async fn before_tool(&self, mut ctx: BeforeToolCtx) -> LoopStep<BeforeToolCtx> {
        if ctx.tool_name == "bash"
            && let Some(command) = ctx.args.get("command").and_then(Value::as_str)
            && self.denies(command)
        {
            ctx.args = serde_json::json!({
                "command": "true",
                "blocked_by": "loop-example",
            });
            return LoopStep::stopped(ctx);
        }
        LoopStep::continued(ctx)
    }

    async fn decide_continue(&self, ctx: DecideContinueCtx) -> LoopStep<DecideContinueCtx> {
        match self.config.max_settles {
            Some(limit) if ctx.settle_count >= limit => LoopStep::stopped(ctx),
            _ => LoopStep::continued(ctx),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct WorkflowProgress {
    total: usize,
    pending: usize,
    in_progress: usize,
    completed: usize,
}

impl WorkflowProgress {
    fn has_unfinished(&self) -> bool {
        self.total > 0 && (self.pending + self.in_progress) > 0
    }
}

pub struct TodoProgressMiddleware {
    progress: Mutex<WorkflowProgress>,
    max_settles: u32,
}

impl TodoProgressMiddleware {
    pub fn new(max_settles: u32) -> Self {
        Self {
            progress: Mutex::new(WorkflowProgress::default()),
            max_settles: max_settles.max(1),
        }
    }

    fn progress(&self) -> WorkflowProgress {
        self.progress
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    fn record_todo_result(&self, ctx: &AfterToolCtx) {
        if ctx.is_error || !matches!(ctx.tool_name.as_str(), "todo_rewrite" | "todo_update") {
            return;
        }
        let Some(text) = ctx.parts.iter().find_map(|part| match part {
            AgentPart::Text { text } => Some(text.as_str()),
            _ => None,
        }) else {
            return;
        };
        let Ok(value) = serde_json::from_str::<Value>(text) else {
            return;
        };
        let Some(items) = value.get("items").and_then(Value::as_array) else {
            return;
        };
        let mut next = WorkflowProgress::default();
        for item in items {
            next.total += 1;
            match item.get("status").and_then(Value::as_str) {
                Some("completed") => next.completed += 1,
                Some("in_progress") => next.in_progress += 1,
                _ => next.pending += 1,
            }
        }
        *self.progress.lock().unwrap_or_else(|e| e.into_inner()) = next;
    }
}

#[async_trait]
impl LoopMiddleware for TodoProgressMiddleware {
    fn id(&self) -> &str {
        "todo-progress"
    }

    fn order(&self) -> i64 {
        200
    }

    fn stages(&self) -> &'static [LoopStage] {
        TODO_PROGRESS_STAGES
    }

    async fn before_turn(&self, mut ctx: BeforeTurnCtx) -> LoopStep<BeforeTurnCtx> {
        ctx.system_fragments.push(TODO_PROGRESS_CONTRACT.into());
        LoopStep::continued(ctx)
    }

    async fn after_tool(&self, ctx: AfterToolCtx) -> LoopStep<AfterToolCtx> {
        self.record_todo_result(&ctx);
        LoopStep::continued(ctx)
    }

    async fn decide_continue(&self, ctx: DecideContinueCtx) -> LoopStep<DecideContinueCtx> {
        let progress = self.progress();
        let should_continue = ctx.has_new_user_messages
            || (progress.has_unfinished() && ctx.settle_count < self.max_settles);
        if should_continue {
            LoopStep::continued(ctx)
        } else {
            LoopStep::stopped(ctx)
        }
    }

    async fn after_turn(&self, ctx: AfterTurnCtx) -> LoopStep<AfterTurnCtx> {
        let progress = self.progress();
        log::info!(
            target: "xylitol::loop-example",
            "todo checkpoint turn={} continued_tools={} completed={}/{}",
            ctx.turn_index,
            ctx.continued_tools,
            progress.completed,
            progress.total
        );
        LoopStep::continued(ctx)
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CriticPhase {
    Idle,
    Critiquing,
    Revising,
    Approved,
    Exhausted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CriticVerdict {
    Approve,
    Revise,
}

#[derive(Debug)]
struct CriticState {
    phase: CriticPhase,
    rounds: u32,
    revision: Option<String>,
    unparsed: u32,
    instruction_injected: bool,
}

impl Default for CriticState {
    fn default() -> Self {
        Self {
            phase: CriticPhase::Idle,
            rounds: 0,
            revision: None,
            unparsed: 0,
            instruction_injected: false,
        }
    }
}

pub struct GeneratorCriticMiddleware {
    rubric: Vec<String>,
    max_rounds: u32,
    state: Mutex<CriticState>,
}

impl GeneratorCriticMiddleware {
    pub fn new(config: &GeneratorCriticConfig) -> Self {
        let rubric = if config.rubric.is_empty() {
            DEFAULT_CRITIC_RUBRIC
                .iter()
                .map(|item| item.to_string())
                .collect()
        } else {
            config.rubric.clone()
        };
        Self {
            rubric,
            max_rounds: config.max_rounds.max(1),
            state: Mutex::new(CriticState::default()),
        }
    }

    fn lock_state(&self) -> std::sync::MutexGuard<'_, CriticState> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn critic_instruction(&self) -> String {
        let rubric = self
            .rubric
            .iter()
            .map(|item| format!("- {item}"))
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "<critic>\nReview the previous assistant answer against this rubric:\n{rubric}\n\
Cite concrete evidence (tool results, file paths or commands). Do not modify files or call tools.\n\
End with exactly one line:\nCRITIC_VERDICT: APPROVE\nor\nCRITIC_VERDICT: REVISE\n</critic>"
        )
    }

    fn revision_instruction(&self, revision: Option<&str>) -> String {
        let revision = revision.unwrap_or("No structured verdict was returned.");
        let mut excerpt: String = revision.chars().take(2000).collect();
        if revision.chars().count() > 2000 {
            excerpt.push('…');
        }
        format!(
            "<critic_revision>\nThe critic requested a revision:\n{excerpt}\n\
Revise the answer and address each issue. Do not claim completion without evidence.\n</critic_revision>"
        )
    }
}

fn assistant_has_tool_calls(message: &AgentMessage) -> bool {
    message
        .content()
        .iter()
        .any(|part| matches!(part, AgentPart::ToolCall { .. }))
}

fn parse_critic_verdict(text: &str) -> Option<CriticVerdict> {
    for line in text.lines() {
        let lower = line.to_ascii_lowercase();
        if let Some(idx) = lower.find("critic_verdict:") {
            let rest = &line[idx + "critic_verdict:".len()..];
            if let Some(verdict) = verdict_from_token(rest.trim()) {
                return Some(verdict);
            }
        }
    }
    None
}

fn verdict_from_token(token: &str) -> Option<CriticVerdict> {
    let token = token
        .trim()
        .trim_matches(|c: char| !c.is_ascii_alphabetic());
    let upper = token.to_ascii_uppercase();
    if upper.starts_with("APPROVE") {
        Some(CriticVerdict::Approve)
    } else if upper.starts_with("REVIS") {
        Some(CriticVerdict::Revise)
    } else {
        None
    }
}

#[async_trait]
impl LoopMiddleware for GeneratorCriticMiddleware {
    fn id(&self) -> &str {
        "generator-critic"
    }

    fn order(&self) -> i64 {
        170
    }

    fn stages(&self) -> &'static [LoopStage] {
        CRITIC_STAGES
    }

    async fn before_model(&self, mut ctx: BeforeModelCtx) -> LoopStep<BeforeModelCtx> {
        let mut state = self.lock_state();
        if state.instruction_injected {
            return LoopStep::continued(ctx);
        }
        match state.phase {
            CriticPhase::Critiquing => {
                ctx.messages.push(AgentMessage::Llm(XyLlmMessage::user(
                    self.critic_instruction(),
                )));
                state.instruction_injected = true;
            }
            CriticPhase::Revising => {
                ctx.messages.push(AgentMessage::Llm(XyLlmMessage::user(
                    self.revision_instruction(state.revision.as_deref()),
                )));
                state.instruction_injected = true;
            }
            _ => {}
        }
        LoopStep::continued(ctx)
    }

    async fn after_model(&self, ctx: AfterModelCtx) -> LoopStep<AfterModelCtx> {
        let mut state = self.lock_state();
        match state.phase {
            CriticPhase::Critiquing => {
                let text = ctx.assistant.text();
                match parse_critic_verdict(&text) {
                    Some(CriticVerdict::Approve) => {
                        state.phase = CriticPhase::Approved;
                        state.revision = None;
                        state.unparsed = 0;
                    }
                    Some(CriticVerdict::Revise) => {
                        state.phase = CriticPhase::Revising;
                        state.revision = Some(text);
                        state.unparsed = 0;
                        state.instruction_injected = false;
                    }
                    None => {
                        state.unparsed += 1;
                        if state.unparsed >= 2 {
                            state.phase = CriticPhase::Exhausted;
                        } else {
                            state.phase = CriticPhase::Revising;
                            state.revision = None;
                            state.instruction_injected = false;
                        }
                    }
                }
            }
            CriticPhase::Idle | CriticPhase::Revising => {
                if assistant_has_tool_calls(&ctx.assistant) {
                    return LoopStep::continued(ctx);
                }
                if state.rounds >= self.max_rounds {
                    state.phase = CriticPhase::Exhausted;
                } else {
                    state.rounds += 1;
                    state.phase = CriticPhase::Critiquing;
                    state.revision = None;
                    state.instruction_injected = false;
                }
            }
            _ => {}
        }
        LoopStep::continued(ctx)
    }

    async fn decide_continue(&self, ctx: DecideContinueCtx) -> LoopStep<DecideContinueCtx> {
        let state = self.lock_state();
        match state.phase {
            CriticPhase::Critiquing | CriticPhase::Revising => LoopStep::continued(ctx),
            CriticPhase::Approved => LoopStep::stopped(ctx),
            CriticPhase::Exhausted => {
                log::warn!(
                    target: "xylitol::loop-example",
                    "generator-critic exhausted after {} round(s)",
                    state.rounds
                );
                LoopStep::stopped(ctx)
            }
            CriticPhase::Idle => LoopStep::continued(ctx),
        }
    }
}

pub fn generator_critic_middleware(config: &GeneratorCriticConfig) -> Arc<dyn LoopMiddleware> {
    Arc::new(GeneratorCriticMiddleware::new(config))
}

#[derive(Clone, Debug)]
struct PendingFailure {
    tool_name: String,
    signature: String,
    attempts: u32,
}

pub struct SelfHealMiddleware {
    signatures: Vec<String>,
    max_attempts: u32,
    pending: Mutex<Option<PendingFailure>>,
}

impl SelfHealMiddleware {
    pub fn new(config: &SelfHealConfig) -> Self {
        let signatures = if config.signatures.is_empty() {
            DEFAULT_SELF_HEAL_SIGNATURES
                .iter()
                .map(|signature| signature.to_string())
                .collect()
        } else {
            config
                .signatures
                .iter()
                .map(|signature| signature.to_lowercase())
                .collect()
        };
        Self {
            signatures,
            max_attempts: config.max_attempts.max(1),
            pending: Mutex::new(None),
        }
    }

    fn lock_pending(&self) -> std::sync::MutexGuard<'_, Option<PendingFailure>> {
        self.pending.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[async_trait]
impl LoopMiddleware for SelfHealMiddleware {
    fn id(&self) -> &str {
        "self-heal"
    }

    fn order(&self) -> i64 {
        210
    }

    fn stages(&self) -> &'static [LoopStage] {
        SELF_HEAL_STAGES
    }

    async fn after_tool(&self, mut ctx: AfterToolCtx) -> LoopStep<AfterToolCtx> {
        let mut pending = self.lock_pending();
        if ctx.is_error {
            let text = preview_text(&ctx.parts, 4000).to_lowercase();
            if let Some(signature) = self
                .signatures
                .iter()
                .find(|signature| text.contains(signature.as_str()))
                .cloned()
            {
                let attempts = match pending.as_ref() {
                    Some(previous)
                        if previous.tool_name == ctx.tool_name
                            && previous.signature == signature =>
                    {
                        previous.attempts.saturating_add(1)
                    }
                    _ => 1,
                };
                *pending = Some(PendingFailure {
                    tool_name: ctx.tool_name.clone(),
                    signature: signature.clone(),
                    attempts,
                });
                ctx.parts.push(AgentPart::text(format!(
                    "<self_heal>\nDeterministic failure detected ({signature}). \\
Do not repeat the same action; diagnose, patch, then re-run the same verification.\n</self_heal>"
                )));
            }
        } else if pending
            .as_ref()
            .is_some_and(|previous| previous.tool_name == ctx.tool_name)
        {
            *pending = None;
        }
        LoopStep::continued(ctx)
    }

    async fn decide_continue(&self, ctx: DecideContinueCtx) -> LoopStep<DecideContinueCtx> {
        let pending = self.lock_pending();
        match pending.as_ref() {
            Some(failure) if failure.attempts >= self.max_attempts => {
                log::warn!(
                    target: "xylitol::loop-example",
                    "self-heal stopped after {} attempts on {} ({})",
                    failure.attempts,
                    failure.tool_name,
                    failure.signature
                );
                LoopStep::stopped(ctx)
            }
            _ => LoopStep::continued(ctx),
        }
    }
}

pub fn self_heal_middleware(config: &SelfHealConfig) -> Arc<dyn LoopMiddleware> {
    Arc::new(SelfHealMiddleware::new(config))
}

#[derive(Default)]
struct CheckpointState {
    steps: Vec<TodoCheckpointStep>,
    last_verified_artifact: Option<String>,
    last_failure: Option<String>,
    attempts: HashMap<String, u32>,
    dirty: bool,
}

pub struct TodoCheckpointMiddleware {
    store: Option<Arc<dyn XySessionStore>>,
    ctx: Option<Ctx>,
    state: Mutex<CheckpointState>,
    restored: AtomicBool,
    injected: AtomicBool,
}

impl TodoCheckpointMiddleware {
    pub fn new(store: Option<Arc<dyn XySessionStore>>, ctx: Option<Ctx>) -> Self {
        Self {
            store,
            ctx,
            state: Mutex::new(CheckpointState::default()),
            restored: AtomicBool::new(false),
            injected: AtomicBool::new(false),
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

    fn lock_state(&self) -> std::sync::MutexGuard<'_, CheckpointState> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn record_todo_result(&self, ctx: &AfterToolCtx) {
        if ctx.is_error || !matches!(ctx.tool_name.as_str(), "todo_rewrite" | "todo_update") {
            return;
        }
        let Some(items) = todo_items_from_parts(&ctx.parts) else {
            return;
        };
        let mut state = self.lock_state();
        let previous: HashMap<String, TodoCheckpointStep> = state
            .steps
            .drain(..)
            .map(|step| (step.id.clone(), step))
            .collect();
        state.steps = items
            .into_iter()
            .map(|(id, content, status)| match previous.get(&id) {
                Some(prev) => TodoCheckpointStep {
                    id,
                    content,
                    status,
                    verification: prev.verification.clone(),
                    output_ref: prev.output_ref.clone(),
                },
                None => TodoCheckpointStep {
                    id,
                    content,
                    status,
                    verification: None,
                    output_ref: None,
                },
            })
            .collect();
        state.dirty = true;
    }

    fn record_tool_outcome(&self, ctx: &AfterToolCtx) {
        let mut state = self.lock_state();
        if ctx.is_error {
            state.last_failure = Some(format!(
                "{}: {}",
                ctx.tool_name,
                preview_text(&ctx.parts, 160)
            ));
            if let Some(step_id) = state
                .steps
                .iter()
                .find(|step| step.status == TodoStatus::InProgress)
                .map(|step| step.id.clone())
            {
                *state.attempts.entry(step_id).or_insert(0) += 1;
            }
            state.dirty = true;
        } else if !matches!(ctx.tool_name.as_str(), "todo_rewrite" | "todo_update") {
            state.last_verified_artifact = Some(format!("{}:{}", ctx.tool_name, ctx.tool_call_id));
        }
    }

    async fn restore(&self, session_id: &str) {
        let Some(store) = self.resolve_store() else {
            return;
        };
        let Ok(branch) = store.load_leaf_branch(session_id).await else {
            return;
        };
        let Some(checkpoint) = latest_todo_checkpoint(&branch) else {
            return;
        };
        let mut state = self.lock_state();
        if !state.steps.is_empty() {
            return;
        }
        state.steps = checkpoint.steps;
        state.last_verified_artifact = checkpoint.last_verified_artifact;
        state.last_failure = checkpoint.last_failure;
        if let Some(cursor) = checkpoint.resume_cursor {
            state
                .attempts
                .insert(cursor.step_id, cursor.attempt.saturating_sub(1));
        }
    }

    fn inject_summary(&self, ctx: &mut BeforeModelCtx) {
        let state = self.lock_state();
        let total = state.steps.len();
        if total == 0 {
            return;
        }
        let unfinished = state
            .steps
            .iter()
            .filter(|step| step.status != TodoStatus::Completed)
            .count();
        if unfinished == 0 {
            return;
        }
        let done = total - unfinished;
        let mut lines = vec![
            "<todo_checkpoint>".to_string(),
            format!("Resuming todo checkpoint: {done}/{total} steps completed."),
        ];
        if let Some(step) = state
            .steps
            .iter()
            .find(|step| step.status != TodoStatus::Completed)
        {
            lines.push(format!("Current step: {} — {}.", step.id, step.content));
        }
        if let Some(failure) = &state.last_failure {
            lines.push(format!("Last failure: {failure}"));
        }
        lines.push(
            "Do not re-run steps already completed with verification; verify prerequisites before continuing."
                .into(),
        );
        lines.push("</todo_checkpoint>".into());
        ctx.messages
            .push(AgentMessage::Llm(XyLlmMessage::user(lines.join("\n"))));
    }

    async fn persist(&self, session_id: Option<&str>) {
        let Some(session_id) = session_id else {
            return;
        };
        let Some(store) = self.resolve_store() else {
            return;
        };
        let checkpoint = {
            let mut state = self.lock_state();
            if !state.dirty {
                return;
            }
            state.dirty = false;
            TodoCheckpoint {
                goal: None,
                steps: state.steps.clone(),
                last_verified_artifact: state.last_verified_artifact.clone(),
                last_failure: state.last_failure.clone(),
                resume_cursor: state
                    .steps
                    .iter()
                    .find(|step| step.status != TodoStatus::Completed)
                    .map(|step| TodoResumeCursor {
                        step_id: step.id.clone(),
                        attempt: state.attempts.get(&step.id).copied().unwrap_or(0) + 1,
                    }),
                updated_at: now_ms(),
            }
        };
        if checkpoint.steps.is_empty() {
            return;
        }
        if let Err(err) = store
            .append_session_entry(session_id, &checkpoint.to_custom_entry_shell())
            .await
        {
            self.lock_state().dirty = true;
            log::warn!(
                target: "xylitol::loop-example",
                "todo checkpoint persist failed: {err}"
            );
        }
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn preview_text(parts: &[AgentPart], limit: usize) -> String {
    let text = parts
        .iter()
        .find_map(|part| match part {
            AgentPart::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .unwrap_or("");
    let mut out: String = text.chars().take(limit).collect();
    if text.chars().count() > limit {
        out.push('…');
    }
    out
}

fn todo_items_from_parts(parts: &[AgentPart]) -> Option<Vec<(String, String, TodoStatus)>> {
    let text = parts.iter().find_map(|part| match part {
        AgentPart::Text { text } => Some(text.as_str()),
        _ => None,
    })?;
    let value = serde_json::from_str::<Value>(text).ok()?;
    let items = value.get("items")?.as_array()?;
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let id = item.get("id")?.as_str()?.to_string();
        let content = item.get("content")?.as_str()?.to_string();
        let status = match item.get("status")?.as_str()? {
            "completed" => TodoStatus::Completed,
            "in_progress" => TodoStatus::InProgress,
            _ => TodoStatus::Pending,
        };
        out.push((id, content, status));
    }
    Some(out)
}

#[async_trait]
impl LoopMiddleware for TodoCheckpointMiddleware {
    fn id(&self) -> &str {
        "todo-checkpoint"
    }

    fn order(&self) -> i64 {
        180
    }

    fn stages(&self) -> &'static [LoopStage] {
        CHECKPOINT_STAGES
    }

    async fn before_model(&self, mut ctx: BeforeModelCtx) -> LoopStep<BeforeModelCtx> {
        if !self.restored.swap(true, Ordering::SeqCst)
            && let Some(session_id) = ctx.session_id.as_deref()
        {
            self.restore(session_id).await;
        }
        if !self.injected.swap(true, Ordering::SeqCst) {
            self.inject_summary(&mut ctx);
        }
        self.persist(ctx.session_id.as_deref()).await;
        LoopStep::continued(ctx)
    }

    async fn after_tool(&self, ctx: AfterToolCtx) -> LoopStep<AfterToolCtx> {
        self.record_todo_result(&ctx);
        self.record_tool_outcome(&ctx);
        LoopStep::continued(ctx)
    }

    async fn after_turn(&self, ctx: AfterTurnCtx) -> LoopStep<AfterTurnCtx> {
        self.persist(ctx.session_id.as_deref()).await;
        LoopStep::continued(ctx)
    }
}

pub fn todo_checkpoint_middleware(
    store: Option<Arc<dyn XySessionStore>>,
    ctx: Option<Ctx>,
) -> Arc<dyn LoopMiddleware> {
    Arc::new(TodoCheckpointMiddleware::new(store, ctx))
}

pub struct ToolExposureMiddleware {
    config: ToolExposureConfig,
}

impl ToolExposureMiddleware {
    pub fn new(config: ToolExposureConfig) -> Self {
        Self { config }
    }

    fn visible(&self, name: &str) -> bool {
        if matches!(name, "todo_rewrite" | "todo_update") {
            return true;
        }
        if self.config.deny.iter().any(|tool| tool == name) {
            return false;
        }
        if self.config.read_only && self.config.write_tools.iter().any(|tool| tool == name) {
            return false;
        }
        if !self.config.allow.is_empty() && !self.config.allow.iter().any(|tool| tool == name) {
            return false;
        }
        true
    }
}

#[async_trait]
impl LoopMiddleware for ToolExposureMiddleware {
    fn id(&self) -> &str {
        "tool-exposure"
    }

    fn order(&self) -> i64 {
        50
    }

    fn stages(&self) -> &'static [LoopStage] {
        TOOL_EXPOSURE_STAGES
    }

    async fn before_model(&self, mut ctx: BeforeModelCtx) -> LoopStep<BeforeModelCtx> {
        if !self.config.enabled {
            return LoopStep::continued(ctx);
        }
        let before = ctx.tool_schemas.len();
        ctx.tool_schemas.retain(|schema| self.visible(&schema.name));
        let after = ctx.tool_schemas.len();
        if after != before {
            log::info!(
                target: "xylitol::loop-example",
                "tool exposure filtered {} of {} tools",
                before - after,
                before
            );
        }
        LoopStep::continued(ctx)
    }
}

pub fn tool_exposure_middleware(config: &ToolExposureConfig) -> Arc<dyn LoopMiddleware> {
    Arc::new(ToolExposureMiddleware::new(config.clone()))
}

pub struct TodoReminderMiddleware {
    stale_after: usize,
    stale_tool_calls: AtomicUsize,
    last_error: AtomicBool,
    last_reminder_turn: AtomicU32,
}

impl TodoReminderMiddleware {
    pub fn new(stale_after: u32) -> Self {
        Self {
            stale_after: stale_after.max(1) as usize,
            stale_tool_calls: AtomicUsize::new(0),
            last_error: AtomicBool::new(false),
            last_reminder_turn: AtomicU32::new(u32::MAX),
        }
    }

    fn claims_completion(message: Option<&AgentMessage>) -> bool {
        let Some(message) = message else {
            return false;
        };
        if message.role_name() != "assistant" {
            return false;
        }
        let text = message.text().to_lowercase();
        ["done", "completed", "finished", "完成", "已完成"]
            .iter()
            .any(|needle| text.contains(needle))
    }

    fn unfinished(todo: &TodoList) -> usize {
        todo.items
            .iter()
            .filter(|item| !matches!(item.status, TodoStatus::Completed))
            .count()
    }
}

#[async_trait]
impl LoopMiddleware for TodoReminderMiddleware {
    fn id(&self) -> &str {
        "todo-reminder"
    }

    fn order(&self) -> i64 {
        150
    }

    fn stages(&self) -> &'static [LoopStage] {
        PROGRESS_STAGES
    }

    async fn after_tool(&self, ctx: AfterToolCtx) -> LoopStep<AfterToolCtx> {
        if matches!(ctx.tool_name.as_str(), "todo_rewrite" | "todo_update") && !ctx.is_error {
            self.stale_tool_calls.store(0, Ordering::SeqCst);
        } else {
            self.stale_tool_calls.fetch_add(1, Ordering::SeqCst);
        }
        self.last_error.store(ctx.is_error, Ordering::SeqCst);
        LoopStep::continued(ctx)
    }

    async fn before_model(&self, mut ctx: BeforeModelCtx) -> LoopStep<BeforeModelCtx> {
        let Some(todo) = &ctx.todo else {
            return LoopStep::continued(ctx);
        };
        let unfinished = Self::unfinished(todo);
        if unfinished == 0 {
            return LoopStep::continued(ctx);
        }
        let turn = ctx.turn_index;
        if self.last_reminder_turn.load(Ordering::SeqCst) == turn {
            return LoopStep::continued(ctx);
        }
        let stale = self.stale_tool_calls.load(Ordering::SeqCst) >= self.stale_after;
        let completion_claim = Self::claims_completion(ctx.messages.last());
        let error = self.last_error.load(Ordering::SeqCst);
        if !(stale || completion_claim || error) {
            return LoopStep::continued(ctx);
        }
        self.last_reminder_turn.store(turn, Ordering::SeqCst);
        ctx.messages
            .push(AgentMessage::Llm(XyLlmMessage::user(format!(
                "<todo_reminder>\n{unfinished} todo item(s) are still unfinished. \
Re-read <todo> and update it with todo_update before claiming completion.\n</todo_reminder>"
            ))));
        LoopStep::continued(ctx)
    }
}

pub fn todo_reminder_middleware(config: &LoopExampleConfig) -> Arc<dyn LoopMiddleware> {
    Arc::new(TodoReminderMiddleware::new(
        config.todo_reminder_stale_after,
    ))
}

pub fn todo_progress_middleware(max_settles: u32) -> Arc<dyn LoopMiddleware> {
    Arc::new(TodoProgressMiddleware::new(max_settles))
}

pub fn example_middleware(config: &LoopExampleConfig) -> Arc<dyn LoopMiddleware> {
    Arc::new(ExampleLoopMiddleware::new(config.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        DEFAULT_CRITIC_ROUNDS, DEFAULT_MAX_REWORKFLOWS, DEFAULT_SELF_HEAL_ATTEMPTS,
        DEFAULT_TODO_REMINDER_STALE_AFTER, GeneratorCriticConfig, SelfHealConfig, WorkflowConfig,
    };
    use xylitol_plugin_sdk::LoopControl;
    use xylitol_plugin_sdk::RuntimeSessionStore;
    use xylitol_plugin_sdk::XySessionError;
    use xylitol_plugin_sdk::XySessionStoreError;
    use xylitol_plugin_sdk::message::XyStopReason;
    use xylitol_plugin_sdk::model::XyToolSchema;
    use xylitol_plugin_sdk::rt::{Ctx, CtxHost};
    use xylitol_plugin_sdk::session::{
        ForkPosition, SessionContext, SessionEntry, TodoItem, latest_todo_checkpoint,
    };

    fn ctx(command: &str) -> BeforeToolCtx {
        BeforeToolCtx {
            turn_index: 0,
            tool_call_id: "call-1".into(),
            tool_name: "bash".into(),
            args: serde_json::json!({ "command": command }),
        }
    }

    #[tokio::test]
    async fn denies_configured_pattern_only() {
        let config = LoopExampleConfig {
            enabled: true,
            max_settles: None,
            deny_bash_patterns: vec!["rm -rf /".into()],
            todo_progress: false,
            todo_reminder: false,
            todo_reminder_stale_after: 3,
            todo_checkpoint: false,
            self_heal: Default::default(),
            generator_critic: Default::default(),
            workflow: Default::default(),
            tool_exposure: Default::default(),
        };
        let mw = ExampleLoopMiddleware::new(config);
        let safe = mw.before_tool(ctx("echo hi")).await;
        assert_eq!(safe.control, LoopControl::Continue);
        let risky = mw.before_tool(ctx("rm -rf / --no-preserve-root")).await;
        assert_eq!(risky.control, LoopControl::Stop);
        assert_eq!(risky.ctx.args["blocked_by"], "loop-example");
    }

    #[tokio::test]
    async fn stops_at_configured_settle_count() {
        let config = LoopExampleConfig {
            enabled: true,
            max_settles: Some(2),
            deny_bash_patterns: Vec::new(),
            todo_progress: false,
            todo_reminder: false,
            todo_reminder_stale_after: 3,
            todo_checkpoint: false,
            self_heal: Default::default(),
            generator_critic: Default::default(),
            workflow: Default::default(),
            tool_exposure: Default::default(),
        };
        let mw = ExampleLoopMiddleware::new(config);
        let base = DecideContinueCtx {
            turn_index: 1,
            settle_count: 1,
            has_new_user_messages: false,
            assistant: None,
            history_len: 2,
            compact: None,
        };
        assert_eq!(
            mw.decide_continue(base.clone()).await.control,
            LoopControl::Continue
        );
        let mut at_limit = base;
        at_limit.settle_count = 2;
        assert_eq!(
            mw.decide_continue(at_limit).await.control,
            LoopControl::Stop
        );
    }

    #[test]
    fn stages_and_identity_are_stable() {
        let mw = ExampleLoopMiddleware::new(LoopExampleConfig::default());
        assert_eq!(mw.id(), "loop-example");
        assert_eq!(mw.order(), 100);
        assert_eq!(mw.stages(), STAGES);
    }

    fn todo_ctx(statuses: &[&str], is_error: bool) -> AfterToolCtx {
        let items: Vec<Value> = statuses
            .iter()
            .enumerate()
            .map(|(i, status)| {
                serde_json::json!({
                    "id": format!("t{i}"),
                    "content": format!("step {i}"),
                    "status": status,
                })
            })
            .collect();
        AfterToolCtx {
            turn_index: 0,
            tool_call_id: "call-1".into(),
            tool_name: "todo_rewrite".into(),
            parts: vec![AgentPart::text(
                serde_json::json!({ "items": items }).to_string(),
            )],
            details: None,
            is_error,
        }
    }

    fn decide_ctx(settle_count: u32, has_new_user_messages: bool) -> DecideContinueCtx {
        DecideContinueCtx {
            turn_index: 0,
            settle_count,
            has_new_user_messages,
            assistant: None,
            history_len: 2,
            compact: None,
        }
    }

    #[tokio::test]
    async fn todo_progress_contract_is_injected() {
        let mw = TodoProgressMiddleware::new(DEFAULT_TODO_PROGRESS_SETTLES);
        let out = mw.before_turn(BeforeTurnCtx::default()).await;
        assert!(
            out.ctx
                .system_fragments
                .iter()
                .any(|fragment| fragment.contains("todo_rewrite")),
            "todo progress must tell the model how to record the checklist"
        );
    }

    #[tokio::test]
    async fn todo_result_updates_progress() {
        let mw = TodoProgressMiddleware::new(DEFAULT_TODO_PROGRESS_SETTLES);
        let out = mw
            .after_tool(todo_ctx(&["completed", "pending"], false))
            .await;
        assert_eq!(out.control, LoopControl::Continue);
        let progress = mw.progress();
        assert_eq!(progress.total, 2);
        assert_eq!(progress.completed, 1);
        assert_eq!(progress.pending, 1);
        assert!(progress.has_unfinished());
    }

    #[tokio::test]
    async fn decide_continue_keeps_unfinished_todo_alive() {
        let mw = TodoProgressMiddleware::new(DEFAULT_TODO_PROGRESS_SETTLES);
        mw.after_tool(todo_ctx(&["pending"], false)).await;
        assert_eq!(
            mw.decide_continue(decide_ctx(0, false)).await.control,
            LoopControl::Continue,
            "an unfinished todo must keep the loop going"
        );
    }

    #[tokio::test]
    async fn decide_continue_stops_when_todo_complete() {
        let mw = TodoProgressMiddleware::new(DEFAULT_TODO_PROGRESS_SETTLES);
        mw.after_tool(todo_ctx(&["completed"], false)).await;
        assert_eq!(
            mw.decide_continue(decide_ctx(0, false)).await.control,
            LoopControl::Stop,
            "a complete todo must not force another model call"
        );
    }

    #[tokio::test]
    async fn decide_continue_stops_at_cap() {
        let mw = TodoProgressMiddleware::new(2);
        mw.after_tool(todo_ctx(&["pending"], false)).await;
        assert_eq!(
            mw.decide_continue(decide_ctx(2, false)).await.control,
            LoopControl::Stop,
            "the todo loop must be bounded"
        );
    }

    #[tokio::test]
    async fn decide_continue_continues_on_new_user() {
        let mw = TodoProgressMiddleware::new(1);
        mw.after_tool(todo_ctx(&["completed"], false)).await;
        assert_eq!(
            mw.decide_continue(decide_ctx(1, true)).await.control,
            LoopControl::Continue,
            "new user input must still be processed"
        );
    }

    #[tokio::test]
    async fn error_or_unknown_tool_does_not_touch_progress() {
        let mw = TodoProgressMiddleware::new(DEFAULT_TODO_PROGRESS_SETTLES);
        mw.after_tool(todo_ctx(&["pending"], true)).await;
        assert_eq!(mw.progress(), WorkflowProgress::default());
        let mut ctx = todo_ctx(&["pending"], false);
        ctx.tool_name = "read".into();
        mw.after_tool(ctx).await;
        assert_eq!(mw.progress(), WorkflowProgress::default());
    }

    #[test]
    fn plan_identity_is_stable() {
        let mw = TodoProgressMiddleware::new(DEFAULT_TODO_PROGRESS_SETTLES);
        assert_eq!(mw.id(), "todo-progress");
        assert_eq!(mw.order(), 200);
        assert_eq!(mw.stages(), TODO_PROGRESS_STAGES);
    }

    fn todo(statuses: &[TodoStatus]) -> TodoList {
        TodoList::new(
            statuses
                .iter()
                .enumerate()
                .map(|(i, status)| TodoItem {
                    id: format!("t{i}"),
                    content: format!("step {i}"),
                    status: *status,
                })
                .collect(),
        )
    }

    fn model_ctx(todo: Option<TodoList>, last: Option<AgentMessage>) -> BeforeModelCtx {
        BeforeModelCtx {
            turn_index: 0,
            messages: last.into_iter().collect(),
            tool_schemas: Vec::new(),
            options: xylitol_plugin_sdk::XyGenerateOptions::default(),
            todo,
            session_id: None,
        }
    }

    fn tool_ctx(name: &str, is_error: bool) -> AfterToolCtx {
        AfterToolCtx {
            turn_index: 0,
            tool_call_id: "call-1".into(),
            tool_name: name.into(),
            parts: Vec::new(),
            details: None,
            is_error,
        }
    }

    #[tokio::test]
    async fn todo_reminder_skips_without_todo() {
        let mw = TodoReminderMiddleware::new(2);
        let out = mw.before_model(model_ctx(None, None)).await;
        assert!(out.ctx.messages.is_empty());
    }

    #[tokio::test]
    async fn todo_reminder_fires_when_stale() {
        let mw = TodoReminderMiddleware::new(2);
        mw.after_tool(tool_ctx("read", false)).await;
        mw.after_tool(tool_ctx("read", false)).await;
        let out = mw
            .before_model(model_ctx(
                Some(todo(&[TodoStatus::Pending, TodoStatus::InProgress])),
                None,
            ))
            .await;
        assert_eq!(out.ctx.messages.len(), 1);
        assert!(out.ctx.messages[0].text().contains("<todo_reminder>"));
    }

    #[tokio::test]
    async fn todo_reminder_resets_on_todo_update() {
        let mw = TodoReminderMiddleware::new(1);
        mw.after_tool(tool_ctx("read", false)).await;
        mw.after_tool(tool_ctx("todo_update", false)).await;
        let out = mw
            .before_model(model_ctx(Some(todo(&[TodoStatus::Pending])), None))
            .await;
        assert!(out.ctx.messages.is_empty());
    }

    #[tokio::test]
    async fn todo_reminder_fires_on_completion_claim() {
        let mw = TodoReminderMiddleware::new(99);
        let claim = AgentMessage::Llm(XyLlmMessage::assistant("all done"));
        let out = mw
            .before_model(model_ctx(Some(todo(&[TodoStatus::Pending])), Some(claim)))
            .await;
        assert_eq!(out.ctx.messages.len(), 2);
        assert!(
            out.ctx
                .messages
                .last()
                .expect("reminder")
                .text()
                .contains("still unfinished")
        );
    }

    #[tokio::test]
    async fn todo_reminder_fires_after_error() {
        let mw = TodoReminderMiddleware::new(99);
        mw.after_tool(tool_ctx("bash", true)).await;
        let out = mw
            .before_model(model_ctx(Some(todo(&[TodoStatus::InProgress])), None))
            .await;
        assert_eq!(out.ctx.messages.len(), 1);
    }

    #[tokio::test]
    async fn todo_reminder_is_at_most_once_per_turn() {
        let mw = TodoReminderMiddleware::new(1);
        mw.after_tool(tool_ctx("read", false)).await;
        let first = mw
            .before_model(model_ctx(Some(todo(&[TodoStatus::Pending])), None))
            .await;
        assert_eq!(first.ctx.messages.len(), 1);
        let second = mw
            .before_model(model_ctx(Some(todo(&[TodoStatus::Pending])), None))
            .await;
        assert!(
            second.ctx.messages.is_empty(),
            "the same turn must not receive two reminders"
        );
    }

    #[tokio::test]
    async fn todo_reminder_skips_when_all_completed() {
        let mw = TodoReminderMiddleware::new(1);
        mw.after_tool(tool_ctx("read", false)).await;
        let out = mw
            .before_model(model_ctx(Some(todo(&[TodoStatus::Completed])), None))
            .await;
        assert!(out.ctx.messages.is_empty());
    }

    #[test]
    fn todo_reminder_identity_is_stable() {
        let mw = TodoReminderMiddleware::new(DEFAULT_TODO_REMINDER_STALE_AFTER);
        assert_eq!(mw.id(), "todo-reminder");
        assert_eq!(mw.order(), 150);
        assert_eq!(mw.stages(), PROGRESS_STAGES);
    }

    fn schema(name: &str) -> XyToolSchema {
        XyToolSchema {
            name: name.into(),
            description: format!("{name} tool"),
            parameters: serde_json::json!({ "type": "object" }),
        }
    }

    fn schemas(names: &[&str]) -> Vec<XyToolSchema> {
        names.iter().map(|name| schema(name)).collect()
    }

    fn exposure_ctx(names: &[&str]) -> BeforeModelCtx {
        BeforeModelCtx {
            turn_index: 0,
            messages: Vec::new(),
            tool_schemas: schemas(names),
            options: xylitol_plugin_sdk::XyGenerateOptions::default(),
            todo: None,
            session_id: None,
        }
    }

    fn visible_names(ctx: &BeforeModelCtx) -> Vec<String> {
        ctx.tool_schemas
            .iter()
            .map(|schema| schema.name.clone())
            .collect()
    }

    #[tokio::test]
    async fn tool_exposure_disabled_keeps_shape() {
        let mw = ToolExposureMiddleware::new(ToolExposureConfig::default());
        let out = mw
            .before_model(exposure_ctx(&["read", "bash", "todo_rewrite"]))
            .await;
        assert_eq!(visible_names(&out.ctx), ["read", "bash", "todo_rewrite"]);
    }

    #[tokio::test]
    async fn tool_exposure_empty_rules_keep_shape() {
        let mw = ToolExposureMiddleware::new(ToolExposureConfig {
            enabled: true,
            ..Default::default()
        });
        let out = mw.before_model(exposure_ctx(&["read", "bash"])).await;
        assert_eq!(visible_names(&out.ctx), ["read", "bash"]);
    }

    #[tokio::test]
    async fn tool_exposure_allow_keeps_todo_tools_visible() {
        let mw = ToolExposureMiddleware::new(ToolExposureConfig {
            enabled: true,
            allow: vec!["read".into()],
            ..Default::default()
        });
        let out = mw
            .before_model(exposure_ctx(&[
                "read",
                "bash",
                "todo_rewrite",
                "todo_update",
            ]))
            .await;
        assert_eq!(
            visible_names(&out.ctx),
            ["read", "todo_rewrite", "todo_update"],
            "todo tools must stay visible so the plan can be updated"
        );
    }

    #[tokio::test]
    async fn tool_exposure_deny_hides_only_listed_tools() {
        let mw = ToolExposureMiddleware::new(ToolExposureConfig {
            enabled: true,
            deny: vec!["bash".into()],
            ..Default::default()
        });
        let out = mw
            .before_model(exposure_ctx(&["read", "bash", "write"]))
            .await;
        assert_eq!(visible_names(&out.ctx), ["read", "write"]);
    }

    #[tokio::test]
    async fn tool_exposure_read_only_hides_write_tools() {
        let mw = ToolExposureMiddleware::new(ToolExposureConfig {
            enabled: true,
            read_only: true,
            write_tools: vec!["write".into(), "edit".into()],
            ..Default::default()
        });
        let out = mw
            .before_model(exposure_ctx(&["read", "write", "edit"]))
            .await;
        assert_eq!(visible_names(&out.ctx), ["read"]);
    }

    #[tokio::test]
    async fn tool_exposure_deny_wins_over_allow() {
        let mw = ToolExposureMiddleware::new(ToolExposureConfig {
            enabled: true,
            allow: vec!["read".into(), "bash".into()],
            deny: vec!["bash".into()],
            ..Default::default()
        });
        let out = mw
            .before_model(exposure_ctx(&["read", "bash", "todo_update"]))
            .await;
        assert_eq!(visible_names(&out.ctx), ["read", "todo_update"]);
    }

    #[test]
    fn tool_exposure_identity_is_stable() {
        let mw = ToolExposureMiddleware::new(ToolExposureConfig::default());
        assert_eq!(mw.id(), "tool-exposure");
        assert_eq!(mw.order(), 50);
        assert_eq!(mw.stages(), TOOL_EXPOSURE_STAGES);
    }

    #[derive(Default)]
    struct FakeStore {
        entries: Mutex<Vec<SessionEntry>>,
    }

    #[async_trait]
    impl XySessionStore for FakeStore {
        async fn exists(&self, _: &str) -> bool {
            true
        }

        async fn load_entries(
            &self,
            _: &str,
        ) -> std::result::Result<Vec<SessionEntry>, XySessionStoreError> {
            Ok(self.entries.lock().unwrap().clone())
        }

        async fn append_session_entry(
            &self,
            _: &str,
            entry: &SessionEntry,
        ) -> std::result::Result<(), XySessionStoreError> {
            self.entries.lock().unwrap().push(entry.clone());
            Ok(())
        }

        async fn build_session_context(
            &self,
            _: &str,
        ) -> std::result::Result<SessionContext, XySessionStoreError> {
            unimplemented!("not used by checkpoint tests")
        }

        async fn create(
            &self,
            _: &str,
            _: Option<&str>,
            _: Option<&str>,
        ) -> std::result::Result<(), XySessionStoreError> {
            Ok(())
        }

        async fn fork(
            &self,
            _: &str,
            _: &str,
            _: &str,
            _: ForkPosition,
        ) -> std::result::Result<(), XySessionError> {
            unimplemented!("not used by checkpoint tests")
        }

        fn set_leaf(&self, _: &str, _: Option<&str>) {}

        fn leaf_id(&self, _: &str) -> Option<String> {
            None
        }
    }

    fn checkpoint_ctx(session_id: &str) -> BeforeModelCtx {
        BeforeModelCtx {
            turn_index: 0,
            messages: Vec::new(),
            tool_schemas: Vec::new(),
            options: xylitol_plugin_sdk::XyGenerateOptions::default(),
            todo: None,
            session_id: Some(session_id.into()),
        }
    }

    #[tokio::test]
    async fn checkpoint_persists_todo_progress() {
        let store = Arc::new(FakeStore::default());
        let mw = TodoCheckpointMiddleware::new(Some(store.clone()), None);
        let first = mw.before_model(checkpoint_ctx("s1")).await;
        assert!(first.ctx.messages.is_empty(), "a fresh run must not inject");
        mw.after_tool(todo_ctx(&["completed", "in_progress"], false))
            .await;
        mw.before_model(checkpoint_ctx("s1")).await;

        let entries = store.entries.lock().unwrap().clone();
        let checkpoint = latest_todo_checkpoint(&entries).expect("persisted checkpoint");
        assert_eq!(checkpoint.steps.len(), 2);
        assert_eq!(checkpoint.unfinished_count(), 1);
        assert_eq!(
            checkpoint
                .resume_cursor
                .as_ref()
                .map(|c| c.step_id.as_str()),
            Some("t1")
        );
    }

    #[tokio::test]
    async fn checkpoint_restore_injects_summary_once() {
        let store = Arc::new(FakeStore::default());
        let mut checkpoint = TodoCheckpoint {
            steps: vec![
                TodoCheckpointStep {
                    id: "t0".into(),
                    content: "step 0".into(),
                    status: TodoStatus::Completed,
                    verification: Some("cargo test -p t0".into()),
                    output_ref: None,
                },
                TodoCheckpointStep {
                    id: "t1".into(),
                    content: "step 1".into(),
                    status: TodoStatus::InProgress,
                    verification: None,
                    output_ref: None,
                },
            ],
            last_failure: Some("bash: tests failed".into()),
            resume_cursor: Some(TodoResumeCursor {
                step_id: "t1".into(),
                attempt: 2,
            }),
            ..TodoCheckpoint::default()
        };
        checkpoint.updated_at = 1;
        store
            .append_session_entry("s1", &checkpoint.to_custom_entry_shell())
            .await
            .unwrap();

        let mw = TodoCheckpointMiddleware::new(Some(store.clone()), None);
        let first = mw.before_model(checkpoint_ctx("s1")).await;
        assert_eq!(first.ctx.messages.len(), 1);
        let text = first.ctx.messages[0].text();
        assert!(text.contains("<todo_checkpoint>"));
        assert!(text.contains("1/2 steps completed"));
        assert!(text.contains("t1"));
        assert!(text.contains("tests failed"));

        let second = mw.before_model(checkpoint_ctx("s1")).await;
        assert_eq!(
            second.ctx.messages.len(),
            0,
            "the summary must be injected at most once"
        );
    }

    #[tokio::test]
    async fn checkpoint_skips_injection_when_complete() {
        let store = Arc::new(FakeStore::default());
        let checkpoint = TodoCheckpoint {
            steps: vec![TodoCheckpointStep {
                id: "t0".into(),
                content: "step 0".into(),
                status: TodoStatus::Completed,
                verification: Some("done".into()),
                output_ref: None,
            }],
            ..TodoCheckpoint::default()
        };
        store
            .append_session_entry("s1", &checkpoint.to_custom_entry_shell())
            .await
            .unwrap();
        let mw = TodoCheckpointMiddleware::new(Some(store), None);
        let out = mw.before_model(checkpoint_ctx("s1")).await;
        assert!(out.ctx.messages.is_empty());
    }

    #[tokio::test]
    async fn checkpoint_records_failure_and_attempt() {
        let store = Arc::new(FakeStore::default());
        let mw = TodoCheckpointMiddleware::new(Some(store.clone()), None);
        mw.before_model(checkpoint_ctx("s1")).await;
        mw.after_tool(todo_ctx(&["in_progress"], false)).await;
        let mut failure = todo_ctx(&["in_progress"], false);
        failure.tool_name = "bash".into();
        failure.parts = vec![AgentPart::text("tests failed")];
        failure.is_error = true;
        mw.after_tool(failure).await;
        mw.before_model(checkpoint_ctx("s1")).await;

        let entries = store.entries.lock().unwrap().clone();
        let checkpoint = latest_todo_checkpoint(&entries).expect("checkpoint");
        assert_eq!(
            checkpoint.last_failure.as_deref(),
            Some("bash: tests failed")
        );
        assert_eq!(checkpoint.resume_cursor.expect("cursor").attempt, 2);
    }

    #[tokio::test]
    async fn checkpoint_without_store_degrades_to_memory() {
        let mw = TodoCheckpointMiddleware::new(None, None);
        mw.before_model(checkpoint_ctx("s1")).await;
        mw.after_tool(todo_ctx(&["pending"], false)).await;
        let out = mw.before_model(checkpoint_ctx("s1")).await;
        assert!(out.ctx.messages.is_empty());
    }

    #[test]
    fn todo_checkpoint_identity_is_stable() {
        let mw = TodoCheckpointMiddleware::new(None, None);
        assert_eq!(mw.id(), "todo-checkpoint");
        assert_eq!(mw.order(), 180);
        assert_eq!(mw.stages(), CHECKPOINT_STAGES);
    }

    fn heal_ctx(name: &str, text: &str, is_error: bool) -> AfterToolCtx {
        AfterToolCtx {
            turn_index: 0,
            tool_call_id: "call-1".into(),
            tool_name: name.into(),
            parts: vec![AgentPart::text(text)],
            details: None,
            is_error,
        }
    }

    #[tokio::test]
    async fn self_heal_detects_deterministic_failure() {
        let mw = SelfHealMiddleware::new(&SelfHealConfig {
            enabled: true,
            ..Default::default()
        });
        let out = mw
            .after_tool(heal_ctx("bash", "test failed: two assertions", true))
            .await;
        assert!(
            out.ctx.parts.iter().any(
                |part| matches!(part, AgentPart::Text { text } if text.contains("<self_heal>"))
            ),
            "a deterministic failure must add repair guidance"
        );
        assert_eq!(
            mw.decide_continue(decide_ctx(0, false)).await.control,
            LoopControl::Continue,
            "a pending failure must force another model call"
        );
    }

    #[tokio::test]
    async fn self_heal_ignores_non_deterministic_failure() {
        let mw = SelfHealMiddleware::new(&SelfHealConfig {
            enabled: true,
            ..Default::default()
        });
        let out = mw
            .after_tool(heal_ctx("bash", "network timeout", true))
            .await;
        assert!(
            !out.ctx.parts.iter().any(
                |part| matches!(part, AgentPart::Text { text } if text.contains("<self_heal>"))
            )
        );
        assert!(mw.pending.lock().unwrap().is_none());
    }

    #[tokio::test]
    async fn self_heal_clears_after_successful_verification() {
        let mw = SelfHealMiddleware::new(&SelfHealConfig {
            enabled: true,
            ..Default::default()
        });
        mw.after_tool(heal_ctx("bash", "tests failed", true)).await;
        assert!(mw.pending.lock().unwrap().is_some());
        mw.after_tool(heal_ctx("bash", "all tests passed", false))
            .await;
        assert!(
            mw.pending.lock().unwrap().is_none(),
            "re-running the failing tool successfully must clear the pending failure"
        );
    }

    #[tokio::test]
    async fn self_heal_stops_at_attempt_cap() {
        let mw = SelfHealMiddleware::new(&SelfHealConfig {
            enabled: true,
            max_attempts: 2,
            ..Default::default()
        });
        mw.after_tool(heal_ctx("bash", "tests failed", true)).await;
        assert_eq!(
            mw.decide_continue(decide_ctx(0, false)).await.control,
            LoopControl::Continue
        );
        mw.after_tool(heal_ctx("bash", "tests failed", true)).await;
        assert_eq!(
            mw.decide_continue(decide_ctx(0, false)).await.control,
            LoopControl::Stop,
            "self-heal must be bounded"
        );
    }

    #[tokio::test]
    async fn self_heal_custom_signature_is_case_insensitive() {
        let mw = SelfHealMiddleware::new(&SelfHealConfig {
            enabled: true,
            signatures: vec!["BUILD BROKEN".into()],
            ..Default::default()
        });
        mw.after_tool(heal_ctx("bash", "build broken: missing symbol", true))
            .await;
        assert!(mw.pending.lock().unwrap().is_some());
    }

    #[test]
    fn self_heal_identity_is_stable() {
        let mw = SelfHealMiddleware::new(&SelfHealConfig::default());
        assert_eq!(mw.id(), "self-heal");
        assert_eq!(mw.order(), 210);
        assert_eq!(mw.stages(), SELF_HEAL_STAGES);
    }

    #[test]
    fn self_heal_default_attempt_cap_is_below_core_limit() {
        assert_eq!(DEFAULT_SELF_HEAL_ATTEMPTS, 3);
        assert!(DEFAULT_SELF_HEAL_ATTEMPTS < 32);
    }

    fn critic_config(max_rounds: u32) -> GeneratorCriticConfig {
        GeneratorCriticConfig {
            enabled: true,
            max_rounds,
            ..Default::default()
        }
    }

    fn model_after(text: &str) -> AfterModelCtx {
        AfterModelCtx {
            turn_index: 0,
            assistant: AgentMessage::Llm(XyLlmMessage::assistant(text)),
            session_id: None,
        }
    }

    fn model_after_tool_call() -> AfterModelCtx {
        AfterModelCtx {
            turn_index: 0,
            assistant: AgentMessage::Llm(XyLlmMessage::AssistantMessage {
                content: vec![AgentPart::ToolCall {
                    id: "call-1".into(),
                    name: "read".into(),
                    arguments: serde_json::json!({ "path": "a.rs" }),
                }],
                stop_reason: Some(XyStopReason::ToolUse),
                usage: None,
                api: String::new(),
                provider: String::new(),
                model: String::new(),
                response_id: None,
                error_message: None,
                timestamp: 0,
                diagnostics: Vec::new(),
            }),
            session_id: None,
        }
    }

    #[tokio::test]
    async fn critic_approve_stops_the_run() {
        let mw = GeneratorCriticMiddleware::new(&critic_config(1));
        mw.after_model(model_after("final answer")).await;
        let injected = mw.before_model(model_ctx(None, None)).await;
        assert_eq!(injected.ctx.messages.len(), 1);
        assert!(injected.ctx.messages[0].text().contains("<critic>"));
        mw.after_model(model_after("CRITIC_VERDICT: APPROVE")).await;
        assert_eq!(
            mw.decide_continue(decide_ctx(0, false)).await.control,
            LoopControl::Stop
        );
    }

    #[tokio::test]
    async fn critic_revise_continues_and_injects_revision() {
        let mw = GeneratorCriticMiddleware::new(&critic_config(1));
        mw.after_model(model_after("final answer")).await;
        mw.before_model(model_ctx(None, None)).await;
        mw.after_model(model_after(
            "CRITIC_VERDICT: REVISE\nThe answer cites no evidence.",
        ))
        .await;
        assert_eq!(
            mw.decide_continue(decide_ctx(0, false)).await.control,
            LoopControl::Continue
        );
        let injected = mw.before_model(model_ctx(None, None)).await;
        assert_eq!(injected.ctx.messages.len(), 1);
        let text = injected.ctx.messages[0].text();
        assert!(text.contains("<critic_revision>"));
        assert!(text.contains("cites no evidence"));
        mw.after_model(model_after("revised answer")).await;
        assert_eq!(
            mw.decide_continue(decide_ctx(0, false)).await.control,
            LoopControl::Stop,
            "max_rounds=1 must stop after one revision"
        );
    }

    #[tokio::test]
    async fn critic_without_verdict_retries_once_then_stops() {
        let mw = GeneratorCriticMiddleware::new(&critic_config(2));
        mw.after_model(model_after("final answer")).await;
        mw.before_model(model_ctx(None, None)).await;
        mw.after_model(model_after("no structured verdict")).await;
        assert_eq!(
            mw.decide_continue(decide_ctx(0, false)).await.control,
            LoopControl::Continue
        );
        mw.before_model(model_ctx(None, None)).await;
        mw.after_model(model_after("revised answer")).await;
        mw.before_model(model_ctx(None, None)).await;
        mw.after_model(model_after("still no verdict")).await;
        assert_eq!(
            mw.decide_continue(decide_ctx(0, false)).await.control,
            LoopControl::Stop,
            "a second missing verdict must stop"
        );
    }

    #[tokio::test]
    async fn critic_ignores_tool_call_turns() {
        let mw = GeneratorCriticMiddleware::new(&critic_config(1));
        mw.after_model(model_after_tool_call()).await;
        let out = mw.before_model(model_ctx(None, None)).await;
        assert!(
            out.ctx.messages.is_empty(),
            "tool-call turns must not start a critique"
        );
        assert_eq!(
            mw.decide_continue(decide_ctx(0, false)).await.control,
            LoopControl::Continue
        );
    }

    #[tokio::test]
    async fn critic_injects_instruction_only_once_per_phase() {
        let mw = GeneratorCriticMiddleware::new(&critic_config(1));
        mw.after_model(model_after("final answer")).await;
        let first = mw.before_model(model_ctx(None, None)).await;
        assert_eq!(first.ctx.messages.len(), 1);
        let second = mw.before_model(model_ctx(None, None)).await;
        assert!(
            second.ctx.messages.is_empty(),
            "the critic instruction must not be repeated within one phase"
        );
    }

    #[test]
    fn critic_identity_is_stable() {
        let mw = GeneratorCriticMiddleware::new(&GeneratorCriticConfig::default());
        assert_eq!(mw.id(), "generator-critic");
        assert_eq!(mw.order(), 170);
        assert_eq!(mw.stages(), CRITIC_STAGES);
    }

    #[test]
    fn critic_default_rounds_are_bounded() {
        assert_eq!(DEFAULT_CRITIC_ROUNDS, 1);
        assert!(DEFAULT_CRITIC_ROUNDS < 32);
    }

    fn exec_config(max_replans: u32) -> WorkflowConfig {
        WorkflowConfig {
            enabled: true,
            max_replans,
        }
    }

    fn plan_step(id: &str, status: WorkflowStepStatus, depends_on: &[&str]) -> WorkflowStep {
        WorkflowStep {
            id: id.into(),
            description: format!("do {id}"),
            depends_on: depends_on.iter().map(|dep| dep.to_string()).collect(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            risk: Default::default(),
            verification: Some(format!("verify {id}")),
            rollback: None,
            requires_human: false,
            status,
            output_ref: None,
        }
    }

    fn plan_with(steps: Vec<WorkflowStep>) -> Workflow {
        Workflow {
            goal: "ship".into(),
            steps,
            resume_cursor: None,
            updated_at: 1,
        }
    }

    fn plan_block(plan: &Workflow) -> String {
        format!("<plan>{}</plan>", plan.to_data_value())
    }

    fn exec_model_ctx(session_id: Option<&str>) -> BeforeModelCtx {
        BeforeModelCtx {
            turn_index: 0,
            messages: Vec::new(),
            tool_schemas: Vec::new(),
            options: xylitol_plugin_sdk::XyGenerateOptions::default(),
            todo: None,
            session_id: session_id.map(str::to_string),
        }
    }

    fn exec_after_model(text: &str) -> AfterModelCtx {
        AfterModelCtx {
            turn_index: 0,
            assistant: AgentMessage::Llm(XyLlmMessage::assistant(text)),
            session_id: Some("s1".into()),
        }
    }

    #[tokio::test]
    async fn workflow_injects_contract_without_plan() {
        let mw = WorkflowMiddleware::new(None, None, &exec_config(DEFAULT_MAX_REWORKFLOWS));
        let out = mw.before_model(exec_model_ctx(Some("s1"))).await;
        assert_eq!(out.ctx.messages.len(), 1);
        assert!(out.ctx.messages[0].text().contains("<plan>{json}</plan>"));
    }

    #[tokio::test]
    async fn workflow_adopts_and_persists_plan() {
        let store = Arc::new(FakeStore::default());
        let mw = WorkflowMiddleware::new(
            Some(store.clone()),
            None,
            &exec_config(DEFAULT_MAX_REWORKFLOWS),
        );
        mw.before_model(exec_model_ctx(Some("s1"))).await;
        let plan = plan_with(vec![
            plan_step("s1", WorkflowStepStatus::Pending, &[]),
            plan_step("s2", WorkflowStepStatus::Pending, &["s1"]),
        ]);
        mw.after_model(exec_after_model(&plan_block(&plan))).await;

        let entries = store.entries.lock().unwrap().clone();
        assert_eq!(latest_workflow(&entries), Some(plan.clone()));

        let out = mw.before_model(exec_model_ctx(Some("s1"))).await;
        assert_eq!(out.ctx.messages.len(), 1);
        let text = out.ctx.messages[0].text();
        assert!(text.contains("<plan_step>"));
        assert!(text.contains("s1"));
    }

    #[tokio::test]
    async fn workflow_marks_failure_and_replans_downstream() {
        let store = Arc::new(FakeStore::default());
        let mw = WorkflowMiddleware::new(Some(store), None, &exec_config(DEFAULT_MAX_REWORKFLOWS));
        mw.before_model(exec_model_ctx(Some("s1"))).await;
        let plan = plan_with(vec![
            plan_step("s1", WorkflowStepStatus::InProgress, &[]),
            plan_step("s2", WorkflowStepStatus::Pending, &["s1"]),
        ]);
        mw.after_model(exec_after_model(&plan_block(&plan))).await;

        let mut failure = heal_ctx("bash", "tests failed", true);
        failure.tool_name = "bash".into();
        mw.after_tool(failure).await;
        assert_eq!(
            mw.decide_continue(decide_ctx(0, false)).await.control,
            LoopControl::Continue
        );
        let replan = mw.before_model(exec_model_ctx(Some("s1"))).await;
        let text = replan.ctx.messages[0].text();
        assert!(text.contains("<plan_replan>"));
        assert!(text.contains("s2"), "downstream step must be named");

        let mut replanned = plan;
        replanned.steps[0].status = WorkflowStepStatus::Pending;
        replanned.steps[0].description = "retry s1".into();
        replanned.steps[1].description = "redo s2".into();
        mw.after_model(exec_after_model(&plan_block(&replanned)))
            .await;
        assert_eq!(
            mw.decide_continue(decide_ctx(0, false)).await.control,
            LoopControl::Continue
        );
        let step = mw.before_model(exec_model_ctx(Some("s1"))).await;
        assert!(step.ctx.messages[0].text().contains("<plan_step>"));
    }

    #[tokio::test]
    async fn workflow_rejects_rewriting_completed_steps() {
        let mw = WorkflowMiddleware::new(None, None, &exec_config(DEFAULT_MAX_REWORKFLOWS));
        mw.before_model(exec_model_ctx(None)).await;
        let plan = plan_with(vec![
            plan_step("s1", WorkflowStepStatus::Completed, &[]),
            plan_step("s2", WorkflowStepStatus::Pending, &["s1"]),
        ]);
        mw.after_model(exec_after_model(&plan_block(&plan))).await;

        let mut rewritten = plan.clone();
        rewritten.steps[0].status = WorkflowStepStatus::Pending;
        rewritten.steps[0].description = "rewrite history".into();
        mw.after_model(exec_after_model(&plan_block(&rewritten)))
            .await;

        let out = mw.before_model(exec_model_ctx(None)).await;
        assert!(
            out.ctx
                .messages
                .iter()
                .any(|message| message.text().contains("<plan_error>"))
        );
        let step = mw.before_model(exec_model_ctx(None)).await;
        assert!(
            step.ctx.messages.is_empty() || !step.ctx.messages[0].text().contains("<plan_step>s1"),
            "the rejected plan must not replace the old plan"
        );
    }

    #[tokio::test]
    async fn workflow_blocks_for_human_and_approves_once() {
        let mw = WorkflowMiddleware::new(None, None, &exec_config(DEFAULT_MAX_REWORKFLOWS));
        mw.before_model(exec_model_ctx(None)).await;
        let mut human = plan_step("s1", WorkflowStepStatus::Pending, &[]);
        human.requires_human = true;
        mw.after_model(exec_after_model(&plan_block(&plan_with(vec![human]))))
            .await;

        assert_eq!(
            mw.decide_continue(decide_ctx(0, false)).await.control,
            LoopControl::Stop
        );
        assert_eq!(
            mw.decide_continue(decide_ctx(0, true)).await.control,
            LoopControl::Continue
        );
        let out = mw.before_model(exec_model_ctx(None)).await;
        assert!(out.ctx.messages[0].text().contains("<plan_step>"));
    }

    #[tokio::test]
    async fn workflow_stops_at_replan_cap() {
        let mw = WorkflowMiddleware::new(None, None, &exec_config(1));
        mw.before_model(exec_model_ctx(None)).await;
        let failed_plan = plan_with(vec![plan_step("s1", WorkflowStepStatus::Failed, &[])]);
        mw.after_model(exec_after_model(&plan_block(&failed_plan)))
            .await;
        assert_eq!(
            mw.decide_continue(decide_ctx(0, false)).await.control,
            LoopControl::Continue
        );
        mw.after_model(exec_after_model(&plan_block(&failed_plan)))
            .await;
        assert_eq!(
            mw.decide_continue(decide_ctx(0, false)).await.control,
            LoopControl::Stop,
            "replanning must be bounded"
        );
    }

    #[tokio::test]
    async fn workflow_summarizes_after_completion() {
        let mw = WorkflowMiddleware::new(None, None, &exec_config(DEFAULT_MAX_REWORKFLOWS));
        mw.before_model(exec_model_ctx(None)).await;
        let plan = plan_with(vec![plan_step("s1", WorkflowStepStatus::Completed, &[])]);
        mw.after_model(exec_after_model(&plan_block(&plan))).await;

        let mut decide = decide_ctx(0, false);
        assert_eq!(
            mw.decide_continue(decide.clone()).await.control,
            LoopControl::Continue
        );
        let out = mw.before_model(exec_model_ctx(None)).await;
        assert!(out.ctx.messages[0].text().contains("<plan_complete>"));
        decide.assistant = Some(AgentMessage::Llm(XyLlmMessage::assistant("done")));
        assert_eq!(mw.decide_continue(decide).await.control, LoopControl::Stop);
    }

    #[tokio::test]
    async fn workflow_resolves_runtime_session_store_from_ctx() {
        let fake = Arc::new(FakeStore::default());
        let ctx = Ctx::root().expect("root");
        ctx.provide(RuntimeSessionStore(fake.clone() as Arc<dyn XySessionStore>))
            .expect("provide runtime store");
        let mw = WorkflowMiddleware::new(None, Some(ctx), &exec_config(DEFAULT_MAX_REWORKFLOWS));
        mw.before_model(exec_model_ctx(Some("s1"))).await;
        let plan = plan_with(vec![plan_step("s1", WorkflowStepStatus::Pending, &[])]);
        mw.after_model(exec_after_model(&plan_block(&plan))).await;
        let entries = fake.entries.lock().unwrap().clone();
        assert_eq!(
            latest_workflow(&entries),
            Some(plan),
            "the middleware must write through the runtime store provided on the context"
        );
    }

    #[test]
    fn workflow_identity_is_stable() {
        let mw = WorkflowMiddleware::new(None, None, &WorkflowConfig::default());
        assert_eq!(mw.id(), "workflow");
        assert_eq!(mw.order(), 190);
        assert_eq!(mw.stages(), WORKFLOW_STAGES);
    }

    #[test]
    fn workflow_default_replans_are_bounded() {
        assert_eq!(DEFAULT_MAX_REWORKFLOWS, 3);
        assert!(DEFAULT_MAX_REWORKFLOWS < 32);
    }
}
