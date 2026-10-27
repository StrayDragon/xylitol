use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::Value;

use xylitol_plugin_sdk::message::{AgentMessage, AgentPart, XyLlmMessage};
use xylitol_plugin_sdk::session::{TodoList, TodoStatus};
use xylitol_plugin_sdk::{
    AfterToolCtx, AfterTurnCtx, BeforeModelCtx, BeforeToolCtx, BeforeTurnCtx, DecideContinueCtx,
    LoopMiddleware, LoopStage, LoopStep,
};

use crate::config::LoopExampleConfig;

const STAGES: &[LoopStage] = &[LoopStage::BeforeTool, LoopStage::DecideContinue];
pub const DEFAULT_PLAN_SETTLES: u32 = 8;
const PLAN_STAGES: &[LoopStage] = &[
    LoopStage::BeforeTurn,
    LoopStage::AfterTool,
    LoopStage::DecideContinue,
    LoopStage::AfterTurn,
];
const PROGRESS_STAGES: &[LoopStage] = &[LoopStage::BeforeModel, LoopStage::AfterTool];
const PLAN_CONTRACT: &str = "Plan mode: before acting, write the checklist with todo_rewrite. \
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
struct PlanProgress {
    total: usize,
    pending: usize,
    in_progress: usize,
    completed: usize,
}

impl PlanProgress {
    fn has_unfinished(&self) -> bool {
        self.total > 0 && (self.pending + self.in_progress) > 0
    }
}

pub struct PlanStateMiddleware {
    progress: Mutex<PlanProgress>,
    max_settles: u32,
}

impl PlanStateMiddleware {
    pub fn new(max_settles: u32) -> Self {
        Self {
            progress: Mutex::new(PlanProgress::default()),
            max_settles: max_settles.max(1),
        }
    }

    fn progress(&self) -> PlanProgress {
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
        let mut next = PlanProgress::default();
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
impl LoopMiddleware for PlanStateMiddleware {
    fn id(&self) -> &str {
        "plan-state"
    }

    fn order(&self) -> i64 {
        200
    }

    fn stages(&self) -> &'static [LoopStage] {
        PLAN_STAGES
    }

    async fn before_turn(&self, mut ctx: BeforeTurnCtx) -> LoopStep<BeforeTurnCtx> {
        ctx.system_fragments.push(PLAN_CONTRACT.into());
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
            "plan checkpoint turn={} continued_tools={} completed={}/{}",
            ctx.turn_index,
            ctx.continued_tools,
            progress.completed,
            progress.total
        );
        LoopStep::continued(ctx)
    }
}

pub struct ProgressReminderMiddleware {
    stale_after: usize,
    stale_tool_calls: AtomicUsize,
    last_error: AtomicBool,
    last_reminder_turn: AtomicU32,
}

impl ProgressReminderMiddleware {
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
impl LoopMiddleware for ProgressReminderMiddleware {
    fn id(&self) -> &str {
        "progress-reminder"
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
                "<progress_reminder>\n{unfinished} plan item(s) are still unfinished. \
Re-read <todo> and update it with todo_update before claiming completion.\n</progress_reminder>"
            ))));
        LoopStep::continued(ctx)
    }
}

pub fn progress_reminder_middleware(config: &LoopExampleConfig) -> Arc<dyn LoopMiddleware> {
    Arc::new(ProgressReminderMiddleware::new(
        config.progress_reminder_stale_after,
    ))
}

pub fn plan_state_middleware(max_settles: u32) -> Arc<dyn LoopMiddleware> {
    Arc::new(PlanStateMiddleware::new(max_settles))
}

pub fn example_middleware(config: &LoopExampleConfig) -> Arc<dyn LoopMiddleware> {
    Arc::new(ExampleLoopMiddleware::new(config.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DEFAULT_PROGRESS_STALE_AFTER;
    use xylitol_plugin_sdk::LoopControl;
    use xylitol_plugin_sdk::session::TodoItem;

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
            plan_state: false,
            progress_reminder: false,

            progress_reminder_stale_after: 3,
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
            plan_state: false,
            progress_reminder: false,

            progress_reminder_stale_after: 3,
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
    async fn plan_contract_is_injected() {
        let mw = PlanStateMiddleware::new(DEFAULT_PLAN_SETTLES);
        let out = mw.before_turn(BeforeTurnCtx::default()).await;
        assert!(
            out.ctx
                .system_fragments
                .iter()
                .any(|fragment| fragment.contains("todo_rewrite")),
            "plan mode must tell the model how to record the plan"
        );
    }

    #[tokio::test]
    async fn todo_result_updates_progress() {
        let mw = PlanStateMiddleware::new(DEFAULT_PLAN_SETTLES);
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
    async fn decide_continue_keeps_unfinished_plan_alive() {
        let mw = PlanStateMiddleware::new(DEFAULT_PLAN_SETTLES);
        mw.after_tool(todo_ctx(&["pending"], false)).await;
        assert_eq!(
            mw.decide_continue(decide_ctx(0, false)).await.control,
            LoopControl::Continue,
            "an unfinished plan must keep the loop going"
        );
    }

    #[tokio::test]
    async fn decide_continue_stops_when_plan_complete() {
        let mw = PlanStateMiddleware::new(DEFAULT_PLAN_SETTLES);
        mw.after_tool(todo_ctx(&["completed"], false)).await;
        assert_eq!(
            mw.decide_continue(decide_ctx(0, false)).await.control,
            LoopControl::Stop,
            "a complete plan must not force another model call"
        );
    }

    #[tokio::test]
    async fn decide_continue_stops_at_cap() {
        let mw = PlanStateMiddleware::new(2);
        mw.after_tool(todo_ctx(&["pending"], false)).await;
        assert_eq!(
            mw.decide_continue(decide_ctx(2, false)).await.control,
            LoopControl::Stop,
            "the plan loop must be bounded"
        );
    }

    #[tokio::test]
    async fn decide_continue_continues_on_new_user() {
        let mw = PlanStateMiddleware::new(1);
        mw.after_tool(todo_ctx(&["completed"], false)).await;
        assert_eq!(
            mw.decide_continue(decide_ctx(1, true)).await.control,
            LoopControl::Continue,
            "new user input must still be processed"
        );
    }

    #[tokio::test]
    async fn error_or_unknown_tool_does_not_touch_progress() {
        let mw = PlanStateMiddleware::new(DEFAULT_PLAN_SETTLES);
        mw.after_tool(todo_ctx(&["pending"], true)).await;
        assert_eq!(mw.progress(), PlanProgress::default());
        let mut ctx = todo_ctx(&["pending"], false);
        ctx.tool_name = "read".into();
        mw.after_tool(ctx).await;
        assert_eq!(mw.progress(), PlanProgress::default());
    }

    #[test]
    fn plan_identity_is_stable() {
        let mw = PlanStateMiddleware::new(DEFAULT_PLAN_SETTLES);
        assert_eq!(mw.id(), "plan-state");
        assert_eq!(mw.order(), 200);
        assert_eq!(mw.stages(), PLAN_STAGES);
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
    async fn progress_reminder_skips_without_todo() {
        let mw = ProgressReminderMiddleware::new(2);
        let out = mw.before_model(model_ctx(None, None)).await;
        assert!(out.ctx.messages.is_empty());
    }

    #[tokio::test]
    async fn progress_reminder_fires_when_stale() {
        let mw = ProgressReminderMiddleware::new(2);
        mw.after_tool(tool_ctx("read", false)).await;
        mw.after_tool(tool_ctx("read", false)).await;
        let out = mw
            .before_model(model_ctx(
                Some(todo(&[TodoStatus::Pending, TodoStatus::InProgress])),
                None,
            ))
            .await;
        assert_eq!(out.ctx.messages.len(), 1);
        assert!(out.ctx.messages[0].text().contains("<progress_reminder>"));
    }

    #[tokio::test]
    async fn progress_reminder_resets_on_todo_update() {
        let mw = ProgressReminderMiddleware::new(1);
        mw.after_tool(tool_ctx("read", false)).await;
        mw.after_tool(tool_ctx("todo_update", false)).await;
        let out = mw
            .before_model(model_ctx(Some(todo(&[TodoStatus::Pending])), None))
            .await;
        assert!(out.ctx.messages.is_empty());
    }

    #[tokio::test]
    async fn progress_reminder_fires_on_completion_claim() {
        let mw = ProgressReminderMiddleware::new(99);
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
    async fn progress_reminder_fires_after_error() {
        let mw = ProgressReminderMiddleware::new(99);
        mw.after_tool(tool_ctx("bash", true)).await;
        let out = mw
            .before_model(model_ctx(Some(todo(&[TodoStatus::InProgress])), None))
            .await;
        assert_eq!(out.ctx.messages.len(), 1);
    }

    #[tokio::test]
    async fn progress_reminder_is_at_most_once_per_turn() {
        let mw = ProgressReminderMiddleware::new(1);
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
    async fn progress_reminder_skips_when_all_completed() {
        let mw = ProgressReminderMiddleware::new(1);
        mw.after_tool(tool_ctx("read", false)).await;
        let out = mw
            .before_model(model_ctx(Some(todo(&[TodoStatus::Completed])), None))
            .await;
        assert!(out.ctx.messages.is_empty());
    }

    #[test]
    fn progress_reminder_identity_is_stable() {
        let mw = ProgressReminderMiddleware::new(DEFAULT_PROGRESS_STALE_AFTER);
        assert_eq!(mw.id(), "progress-reminder");
        assert_eq!(mw.order(), 150);
        assert_eq!(mw.stages(), PROGRESS_STAGES);
    }
}
