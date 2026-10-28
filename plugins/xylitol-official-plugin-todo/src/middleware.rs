use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::Value;

use xylitol_plugin_sdk::message::{AgentMessage, AgentPart, XyLlmMessage};
use xylitol_plugin_sdk::rt::Ctx;
use xylitol_plugin_sdk::session::{
    TodoCheckpoint, TodoCheckpointStep, TodoList, TodoResumeCursor, TodoStatus,
    latest_todo_checkpoint,
};
use xylitol_plugin_sdk::{
    AfterToolCtx, AfterTurnCtx, BeforeLlmRequestCtx, BeforeTurnCtx, DecideContinueCtx,
    LoopMiddleware, LoopStage, LoopStep, RuntimeSessionStore, XySessionStore,
};

use crate::config::TodoConfig;

pub const DEFAULT_TODO_PROGRESS_SETTLES: u32 = 8;
const TODO_PROGRESS_STAGES: &[LoopStage] = &[
    LoopStage::BeforeTurn,
    LoopStage::AfterTool,
    LoopStage::DecideContinue,
    LoopStage::AfterTurn,
];
const PROGRESS_STAGES: &[LoopStage] = &[LoopStage::BeforeLlmRequest, LoopStage::AfterTool];
const CHECKPOINT_STAGES: &[LoopStage] = &[
    LoopStage::BeforeLlmRequest,
    LoopStage::AfterTool,
    LoopStage::AfterTurn,
];
const TODO_PROGRESS_CONTRACT: &str = "Todo progress: before acting, write the checklist with todo_rewrite. \
Keep exactly one item in_progress and mark finished items with todo_update. \
Do not claim completion while items remain pending.";

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
        let unfinished = progress.pending + progress.in_progress;
        let should_continue = ctx.has_new_user_messages
            || (progress.has_unfinished() && ctx.settle_count < self.max_settles);
        let step = if should_continue {
            LoopStep::continued(ctx)
        } else {
            LoopStep::stopped(ctx)
        };
        step.with_properties([
            (
                "loop.middleware.todo.pending".to_string(),
                unfinished.to_string(),
            ),
            (
                "loop.middleware.todo.total".to_string(),
                progress.total.to_string(),
            ),
        ])
    }

    async fn after_turn(&self, ctx: AfterTurnCtx) -> LoopStep<AfterTurnCtx> {
        let progress = self.progress();
        log::info!(
            target: "xylitol::loop-policies",
            "todo checkpoint turn={} continued_tools={} completed={}/{}",
            ctx.turn_index,
            ctx.continued_tools,
            progress.completed,
            progress.total
        );
        LoopStep::continued(ctx)
    }
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

    fn inject_summary(&self, ctx: &mut BeforeLlmRequestCtx) {
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
                target: "xylitol::loop-policies",
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

    async fn before_llm_request(
        &self,
        mut ctx: BeforeLlmRequestCtx,
    ) -> LoopStep<BeforeLlmRequestCtx> {
        let restored =
            !self.restored.swap(true, Ordering::SeqCst) && ctx.session_id.as_deref().is_some();
        if restored && let Some(session_id) = ctx.session_id.as_deref() {
            self.restore(session_id).await;
        }
        let injected = !self.injected.swap(true, Ordering::SeqCst);
        if injected {
            self.inject_summary(&mut ctx);
        }
        self.persist(ctx.session_id.as_deref()).await;
        let state = if restored {
            "restored"
        } else if injected {
            "injected"
        } else {
            "persisted"
        };
        LoopStep::continued(ctx).with_property("loop.middleware.todo.checkpoint", state)
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

    async fn before_llm_request(
        &self,
        mut ctx: BeforeLlmRequestCtx,
    ) -> LoopStep<BeforeLlmRequestCtx> {
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
        let reason = if error {
            "tool_error"
        } else if completion_claim {
            "completion_claim"
        } else {
            "stale"
        };
        let total = todo.items.len();
        LoopStep::continued(ctx).with_properties([
            (
                "loop.middleware.todo.reminder_reason".to_string(),
                reason.to_string(),
            ),
            (
                "loop.middleware.todo.pending".to_string(),
                unfinished.to_string(),
            ),
            ("loop.middleware.todo.total".to_string(), total.to_string()),
        ])
    }
}

pub fn todo_reminder_middleware(config: &TodoConfig) -> Arc<dyn LoopMiddleware> {
    Arc::new(TodoReminderMiddleware::new(config.reminder_stale_after))
}

pub fn todo_progress_middleware(max_settles: u32) -> Arc<dyn LoopMiddleware> {
    Arc::new(TodoProgressMiddleware::new(max_settles))
}
