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

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;

    use super::*;
    use xylitol_plugin_sdk::session::{
        ForkPosition, SessionContext, SessionEntry, TodoCheckpoint, TodoCheckpointStep, TodoItem,
    };
    use xylitol_plugin_sdk::{LoopControl, XySessionError, XySessionStoreError};

    #[derive(Default)]
    struct MemStore {
        entries: Mutex<Vec<SessionEntry>>,
    }

    #[async_trait]
    impl XySessionStore for MemStore {
        async fn exists(&self, _: &str) -> bool {
            true
        }
        async fn load_entries(&self, _: &str) -> Result<Vec<SessionEntry>, XySessionStoreError> {
            Ok(self.entries.lock().unwrap().clone())
        }
        async fn append_session_entry(
            &self,
            _: &str,
            entry: &SessionEntry,
        ) -> Result<(), XySessionStoreError> {
            self.entries.lock().unwrap().push(entry.clone());
            Ok(())
        }
        async fn build_session_context(
            &self,
            _: &str,
        ) -> Result<SessionContext, XySessionStoreError> {
            Err(XySessionStoreError::unsupported("build_session_context"))
        }
        async fn create(
            &self,
            _: &str,
            _: Option<&str>,
            _: Option<&str>,
        ) -> Result<(), XySessionStoreError> {
            Ok(())
        }
        async fn fork(
            &self,
            _: &str,
            _: &str,
            at_entry_id: &str,
            _: ForkPosition,
        ) -> Result<(), XySessionError> {
            Err(XySessionError::EntryNotFound {
                entry_id: at_entry_id.into(),
            })
        }
        fn set_leaf(&self, _: &str, _: Option<&str>) {}
        fn leaf_id(&self, _: &str) -> Option<String> {
            None
        }
    }

    fn todo(items: &[(&str, TodoStatus)]) -> TodoList {
        TodoList::new(
            items
                .iter()
                .map(|(id, status)| TodoItem {
                    id: (*id).into(),
                    content: format!("do {id}"),
                    status: *status,
                })
                .collect(),
        )
    }

    fn todo_json(items: &[(&str, TodoStatus)]) -> String {
        serde_json::json!({
            "items": items.iter().map(|(id, status)| serde_json::json!({
                "id": id,
                "content": format!("do {id}"),
                "status": match status {
                    TodoStatus::Completed => "completed",
                    TodoStatus::InProgress => "in_progress",
                    TodoStatus::Pending => "pending",
                },
            })).collect::<Vec<_>>()
        })
        .to_string()
    }

    fn after_tool(name: &str, text: &str, is_error: bool) -> AfterToolCtx {
        AfterToolCtx {
            turn_index: 0,
            tool_call_id: "call-1".into(),
            tool_name: name.into(),
            parts: vec![AgentPart::text(text)],
            details: None,
            is_error,
        }
    }

    fn model_ctx(
        todo: Option<TodoList>,
        last: Option<AgentMessage>,
        session_id: Option<&str>,
    ) -> BeforeLlmRequestCtx {
        BeforeLlmRequestCtx {
            turn_index: 1,
            messages: last.into_iter().collect(),
            tool_schemas: Vec::new(),
            options: xylitol_plugin_sdk::XyGenerateOptions::default(),
            todo,
            plan_mode: None,
            session_id: session_id.map(str::to_string),
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

    #[test]
    fn todo_items_from_parts_parses_statuses() {
        let parts = vec![AgentPart::text(todo_json(&[
            ("a", TodoStatus::Completed),
            ("b", TodoStatus::InProgress),
            ("c", TodoStatus::Pending),
        ]))];
        let items = todo_items_from_parts(&parts).expect("items");
        assert_eq!(items[0].2, TodoStatus::Completed);
        assert_eq!(items[1].2, TodoStatus::InProgress);
        assert_eq!(items[2].2, TodoStatus::Pending);
    }

    #[test]
    fn todo_items_from_parts_rejects_bad_payloads() {
        assert!(todo_items_from_parts(&[AgentPart::text("not json")]).is_none());
        assert!(todo_items_from_parts(&[AgentPart::text("{}")]).is_none());
        assert!(
            todo_items_from_parts(&[AgentPart::text(r#"{"items":[{"id":"a"}]}"#)]).is_none(),
            "missing content must not be guessed"
        );
    }

    #[tokio::test]
    async fn progress_tracks_todo_results_and_decides_continue() {
        let mw = TodoProgressMiddleware::new(8);
        let out = mw.before_turn(BeforeTurnCtx::default()).await;
        assert!(
            out.ctx
                .system_fragments
                .iter()
                .any(|f| f.contains("todo_rewrite"))
        );

        mw.after_tool(after_tool(
            "todo_rewrite",
            &todo_json(&[
                ("a", TodoStatus::Completed),
                ("b", TodoStatus::Pending),
                ("c", TodoStatus::InProgress),
            ]),
            false,
        ))
        .await;
        let out = mw.decide_continue(decide_ctx(0, false)).await;
        assert_eq!(out.control, LoopControl::Continue);
        assert_eq!(out.properties[0].1, "2", "pending + in_progress");

        let out = mw.decide_continue(decide_ctx(8, false)).await;
        assert_eq!(out.control, LoopControl::Stop);

        let out = mw.decide_continue(decide_ctx(8, true)).await;
        assert_eq!(
            out.control,
            LoopControl::Continue,
            "new user messages always continue"
        );
    }

    #[tokio::test]
    async fn progress_ignores_errors_and_unrelated_tools() {
        let mw = TodoProgressMiddleware::new(8);
        mw.after_tool(after_tool(
            "todo_rewrite",
            &todo_json(&[("a", TodoStatus::Pending)]),
            true,
        ))
        .await;
        mw.after_tool(after_tool("read", "ok", false)).await;
        let out = mw.decide_continue(decide_ctx(0, false)).await;
        assert_eq!(
            out.control,
            LoopControl::Stop,
            "no recorded unfinished todo means stop"
        );
    }

    #[tokio::test]
    async fn checkpoint_records_and_persists_once() {
        let store: Arc<dyn XySessionStore> = Arc::new(MemStore::default());
        let mw = todo_checkpoint_middleware(Some(store.clone()), None);
        mw.after_tool(after_tool(
            "todo_rewrite",
            &todo_json(&[("a", TodoStatus::Completed), ("b", TodoStatus::InProgress)]),
            false,
        ))
        .await;
        let out = mw
            .before_llm_request(model_ctx(None, None, Some("s1")))
            .await;
        assert_eq!(
            out.properties
                .iter()
                .find(|(k, _)| k == "loop.middleware.todo.checkpoint")
                .map(|(_, v)| v.as_str()),
            Some("restored")
        );
        let text = out
            .ctx
            .messages
            .iter()
            .map(|m| m.text())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("<todo_checkpoint>"), "{text}");
        assert!(text.contains("1/2 steps completed"), "{text}");

        let entries = store.load_entries("s1").await.unwrap();
        let checkpoint = entries
            .iter()
            .find_map(|entry| match entry {
                SessionEntry::Custom(custom) if custom.custom_type == "todo_checkpoint" => {
                    Some(custom.clone())
                }
                _ => None,
            })
            .expect("checkpoint appended");
        assert_eq!(checkpoint.data["steps"][1]["status"], "in_progress");
    }

    #[tokio::test]
    async fn checkpoint_restores_and_injects_summary_once() {
        let store = Arc::new(MemStore::default());
        let checkpoint = TodoCheckpoint {
            goal: Some("ship".into()),
            steps: vec![TodoCheckpointStep {
                id: "s1".into(),
                content: "do it".into(),
                status: TodoStatus::InProgress,
                verification: Some("tests".into()),
                output_ref: None,
            }],
            last_verified_artifact: None,
            last_failure: Some("read: missing".into()),
            resume_cursor: Some(TodoResumeCursor {
                step_id: "s1".into(),
                attempt: 2,
            }),
            updated_at: 1,
        };
        store
            .append_session_entry("s1", &checkpoint.to_custom_entry_shell())
            .await
            .unwrap();
        let mw = todo_checkpoint_middleware(Some(store), None);
        let out = mw
            .before_llm_request(model_ctx(None, None, Some("s1")))
            .await;
        let text = out
            .ctx
            .messages
            .iter()
            .map(|m| m.text())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("Current step: s1"), "{text}");
        assert!(text.contains("Last failure: read: missing"), "{text}");

        let out = mw
            .before_llm_request(model_ctx(None, None, Some("s1")))
            .await;
        assert!(
            !out.ctx
                .messages
                .iter()
                .any(|m| m.text().contains("<todo_checkpoint>")),
            "the summary must be injected once"
        );
    }

    #[tokio::test]
    async fn checkpoint_records_failures_and_verified_artifacts() {
        let store: Arc<dyn XySessionStore> = Arc::new(MemStore::default());
        let mw = todo_checkpoint_middleware(Some(store.clone()), None);
        mw.after_tool(after_tool(
            "todo_rewrite",
            &todo_json(&[("a", TodoStatus::InProgress)]),
            false,
        ))
        .await;
        mw.after_tool(after_tool("write", "permission denied", true))
            .await;
        mw.after_tool(after_tool("read", "ok", false)).await;
        mw.after_turn(AfterTurnCtx {
            turn_index: 1,
            continued_tools: false,
            history_len: 4,
            session_id: Some("s1".into()),
        })
        .await;

        let entries = store.load_entries("s1").await.unwrap();
        let checkpoint = entries
            .iter()
            .find_map(|entry| match entry {
                SessionEntry::Custom(custom) if custom.custom_type == "todo_checkpoint" => {
                    Some(custom.clone())
                }
                _ => None,
            })
            .expect("checkpoint appended");
        assert_eq!(checkpoint.data["steps"][0]["status"], "in_progress");
        assert_eq!(checkpoint.data["resume_cursor"]["attempt"], 2);
    }

    #[tokio::test]
    async fn reminder_triggers_on_stale_completion_and_error() {
        let mw = TodoReminderMiddleware::new(2);
        let pending = todo(&[("a", TodoStatus::Pending)]);
        let out = mw
            .before_llm_request(model_ctx(Some(pending.clone()), None, None))
            .await;
        assert!(
            !out.ctx
                .messages
                .iter()
                .any(|m| m.text().contains("<todo_reminder>"))
        );

        mw.after_tool(after_tool("read", "ok", false)).await;
        mw.after_tool(after_tool("grep", "ok", false)).await;
        let out = mw
            .before_llm_request(model_ctx(Some(pending.clone()), None, None))
            .await;
        assert!(
            out.ctx
                .messages
                .iter()
                .any(|m| m.text().contains("<todo_reminder>"))
        );
        assert_eq!(
            out.properties
                .iter()
                .find(|(k, _)| k == "loop.middleware.todo.reminder_reason")
                .map(|(_, v)| v.as_str()),
            Some("stale")
        );

        let out = mw
            .before_llm_request(model_ctx(Some(pending.clone()), None, None))
            .await;
        assert!(
            !out.ctx
                .messages
                .iter()
                .any(|m| m.text().contains("<todo_reminder>")),
            "the same turn must not repeat the reminder"
        );
    }

    #[tokio::test]
    async fn reminder_triggers_on_completion_claim_and_error() {
        let mw = TodoReminderMiddleware::new(8);
        let pending = todo(&[("a", TodoStatus::Pending)]);
        let out = mw
            .before_llm_request(model_ctx(
                Some(pending.clone()),
                Some(AgentMessage::assistant("all done")),
                None,
            ))
            .await;
        assert_eq!(
            out.properties
                .iter()
                .find(|(k, _)| k == "loop.middleware.todo.reminder_reason")
                .map(|(_, v)| v.as_str()),
            Some("completion_claim")
        );

        let mw = TodoReminderMiddleware::new(8);
        mw.after_tool(after_tool("write", "failed", true)).await;
        let out = mw
            .before_llm_request(model_ctx(Some(pending), None, None))
            .await;
        assert_eq!(
            out.properties
                .iter()
                .find(|(k, _)| k == "loop.middleware.todo.reminder_reason")
                .map(|(_, v)| v.as_str()),
            Some("tool_error")
        );
    }

    #[tokio::test]
    async fn reminder_skips_empty_and_completed_todos() {
        let mw = TodoReminderMiddleware::new(1);
        let out = mw.before_llm_request(model_ctx(None, None, None)).await;
        assert!(out.ctx.messages.is_empty());

        let done = todo(&[("a", TodoStatus::Completed)]);
        let out = mw
            .before_llm_request(model_ctx(Some(done), None, None))
            .await;
        assert!(out.ctx.messages.is_empty());
    }

    #[tokio::test]
    async fn reminder_resets_stale_counter_on_todo_update() {
        let mw = TodoReminderMiddleware::new(2);
        mw.after_tool(after_tool("read", "ok", false)).await;
        mw.after_tool(after_tool(
            "todo_update",
            &todo_json(&[("a", TodoStatus::Pending)]),
            false,
        ))
        .await;
        let out = mw
            .before_llm_request(model_ctx(
                Some(todo(&[("a", TodoStatus::Pending)])),
                None,
                None,
            ))
            .await;
        assert!(
            !out.ctx
                .messages
                .iter()
                .any(|m| m.text().contains("<todo_reminder>")),
            "a todo update resets the stale counter"
        );
    }
}
