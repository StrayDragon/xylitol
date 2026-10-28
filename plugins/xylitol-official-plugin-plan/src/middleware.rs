use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use xylitol_plugin_sdk::message::{AgentMessage, XyLlmMessage};
use xylitol_plugin_sdk::rt::Ctx;
use xylitol_plugin_sdk::session::{PlanMode, PlanModeKind, latest_plan_mode};
use xylitol_plugin_sdk::{
    BeforeLlmRequestCtx, BeforeToolCtx, LoopMiddleware, LoopStage, LoopStep, RuntimeSessionStore,
    XySessionStore,
};

use crate::config::PlanConfig;

const PLAN_MODE_STAGES: &[LoopStage] = &[LoopStage::BeforeLlmRequest, LoopStage::BeforeTool];
const READ_ONLY_ALLOWLIST: &[&str] = &[
    "read",
    "grep",
    "find",
    "ls",
    "ask",
    "plan_write",
    "plan_read",
    "plan_list",
    "plan_submit",
    "plan_mode",
    "todo_rewrite",
    "todo_update",
];
const PLAN_MODE_CONTRACT: &str = "Plan mode: research and read first. Do not edit files yet. \
Write the plan with plan_write, then call plan_submit for approval. \
After approval the mode switches to execute and the todo list is seeded.";

pub struct PlanModeMiddleware {
    store: Option<Arc<dyn XySessionStore>>,
    ctx: Option<Ctx>,
    mode: Mutex<PlanMode>,
    read_only: bool,
    allowed_tools: Vec<String>,
}

impl PlanModeMiddleware {
    pub fn new(
        store: Option<Arc<dyn XySessionStore>>,
        ctx: Option<Ctx>,
        config: &PlanConfig,
    ) -> Self {
        Self {
            store,
            ctx,
            mode: Mutex::new(PlanMode::execute()),
            read_only: config.read_only,
            allowed_tools: config.allowed_tools.clone(),
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

    fn allows(&self, tool_name: &str) -> bool {
        READ_ONLY_ALLOWLIST.contains(&tool_name)
            || self.allowed_tools.iter().any(|tool| tool == tool_name)
    }

    async fn load_mode(&self, session_id: Option<&str>) {
        let (Some(store), Some(session_id)) = (self.resolve_store(), session_id) else {
            return;
        };
        let Ok(branch) = store.load_leaf_branch(session_id).await else {
            return;
        };
        let mode = latest_plan_mode(&branch);
        *self.mode.lock().unwrap_or_else(|e| e.into_inner()) = mode;
    }
}

#[async_trait]
impl LoopMiddleware for PlanModeMiddleware {
    fn id(&self) -> &str {
        "plan-mode"
    }

    fn order(&self) -> i64 {
        185
    }

    fn stages(&self) -> &'static [LoopStage] {
        PLAN_MODE_STAGES
    }

    async fn before_llm_request(&self, ctx: BeforeLlmRequestCtx) -> LoopStep<BeforeLlmRequestCtx> {
        self.load_mode(ctx.session_id.as_deref()).await;
        let mode = self.mode.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let mut step = LoopStep::continued(ctx);
        if mode.mode == PlanModeKind::Plan {
            step.ctx
                .messages
                .push(AgentMessage::Llm(XyLlmMessage::user(PLAN_MODE_CONTRACT)));
            step = step.with_property("loop.middleware.plan.mode", "plan");
            if let Some(plan_id) = &mode.plan_id {
                step = step.with_property("loop.middleware.plan.id", plan_id.clone());
            }
        }
        step
    }

    async fn before_tool(&self, mut ctx: BeforeToolCtx) -> LoopStep<BeforeToolCtx> {
        if !self.read_only {
            return LoopStep::continued(ctx);
        }
        let mode = self.mode.lock().unwrap_or_else(|e| e.into_inner()).clone();
        if mode.mode != PlanModeKind::Plan || self.allows(&ctx.tool_name) {
            return LoopStep::continued(ctx);
        }
        let denied_tool = ctx.tool_name.clone();
        ctx.args = serde_json::json!({
            "blocked_by": "plan-mode",
            "reason": "plan mode is read-only",
        });
        LoopStep::stopped(ctx)
            .with_property("loop.middleware.plan.read_only", "true")
            .with_property("loop.middleware.plan.denied_tool", denied_tool)
    }
}

pub fn plan_mode_middleware(
    store: Option<Arc<dyn XySessionStore>>,
    ctx: Option<Ctx>,
    config: &PlanConfig,
) -> Arc<dyn LoopMiddleware> {
    Arc::new(PlanModeMiddleware::new(store, ctx, config))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use async_trait::async_trait;

    use super::*;
    use xylitol_plugin_sdk::session::{ForkPosition, SessionContext, SessionEntry};
    use xylitol_plugin_sdk::{LoopControl, XySessionError, XySessionStoreError};

    #[derive(Default)]
    struct FakeStore {
        entries: Mutex<HashMap<String, Vec<SessionEntry>>>,
    }

    #[async_trait]
    impl XySessionStore for FakeStore {
        async fn exists(&self, session_id: &str) -> bool {
            self.entries
                .lock()
                .map(|m| m.contains_key(session_id))
                .unwrap_or(false)
        }

        async fn load_entries(
            &self,
            session_id: &str,
        ) -> Result<Vec<SessionEntry>, XySessionStoreError> {
            self.entries
                .lock()
                .ok()
                .and_then(|m| m.get(session_id).cloned())
                .ok_or_else(|| XySessionStoreError::not_found(session_id))
        }

        async fn append_session_entry(
            &self,
            session_id: &str,
            entry: &SessionEntry,
        ) -> Result<(), XySessionStoreError> {
            self.entries
                .lock()
                .map_err(|_| XySessionStoreError::io("lock", std::io::Error::other("poison")))?
                .entry(session_id.to_string())
                .or_default()
                .push(entry.clone());
            Ok(())
        }

        async fn build_session_context(
            &self,
            session_id: &str,
        ) -> Result<SessionContext, XySessionStoreError> {
            let _ = self.load_entries(session_id).await?;
            Ok(SessionContext {
                messages: Vec::new(),
                thinking_level: String::new(),
                model: None,
            })
        }

        async fn create(
            &self,
            id: &str,
            _cwd: Option<&str>,
            _parent: Option<&str>,
        ) -> Result<(), XySessionStoreError> {
            self.entries
                .lock()
                .map_err(|_| XySessionStoreError::io("lock", std::io::Error::other("poison")))?
                .entry(id.to_string())
                .or_default();
            Ok(())
        }

        async fn fork(
            &self,
            _parent_id: &str,
            _child_id: &str,
            at_entry_id: &str,
            _position: ForkPosition,
        ) -> Result<(), XySessionError> {
            Err(XySessionError::EntryNotFound {
                entry_id: at_entry_id.into(),
            })
        }

        fn set_leaf(&self, _session_id: &str, _entry_id: Option<&str>) {}

        fn leaf_id(&self, _session_id: &str) -> Option<String> {
            None
        }
    }

    fn model_ctx(session_id: &str) -> BeforeLlmRequestCtx {
        BeforeLlmRequestCtx {
            turn_index: 0,
            messages: Vec::new(),
            tool_schemas: Vec::new(),
            options: xylitol_plugin_sdk::XyGenerateOptions::default(),
            todo: None,
            session_id: Some(session_id.into()),
        }
    }

    fn tool_ctx(name: &str) -> BeforeToolCtx {
        BeforeToolCtx {
            turn_index: 0,
            tool_call_id: "call-1".into(),
            tool_name: name.into(),
            args: serde_json::json!({ "path": "src/lib.rs" }),
        }
    }

    async fn plan_middleware(store: Arc<FakeStore>, config: PlanConfig) -> Arc<dyn LoopMiddleware> {
        store
            .append_session_entry("s1", &PlanMode::plan(None).to_custom_entry_shell())
            .await
            .expect("plan mode entry");
        let mw = plan_mode_middleware(Some(store), None, &config);
        mw.before_llm_request(model_ctx("s1")).await;
        mw
    }

    #[tokio::test]
    async fn plan_gate_denies_mutating_and_unknown_tools() {
        let mw = plan_middleware(Arc::new(FakeStore::default()), PlanConfig::default()).await;
        for name in ["write", "edit", "bash", "mcp__write", "unknown_tool"] {
            let out = mw.before_tool(tool_ctx(name)).await;
            assert_eq!(out.control, LoopControl::Stop, "{name} must be denied");
            assert_eq!(out.ctx.args["blocked_by"], "plan-mode");
            assert_eq!(out.ctx.args["reason"], "plan mode is read-only");
            assert!(
                out.properties.iter().any(|(key, value)| {
                    key == "loop.middleware.plan.denied_tool" && value == name
                }),
                "{name} must report the denied tool"
            );
            assert!(
                out.properties
                    .iter()
                    .any(|(key, value)| key == "loop.middleware.plan.read_only" && value == "true"),
                "denial must report read-only"
            );
        }
    }

    #[tokio::test]
    async fn plan_mode_reports_mode_and_plan_id() {
        use xylitol_plugin_sdk::session::PlanMode;

        let store = Arc::new(FakeStore::default());
        store
            .append_session_entry(
                "s1",
                &PlanMode::plan(Some("p1".into())).to_custom_entry_shell(),
            )
            .await
            .expect("plan mode entry");
        let mw = plan_mode_middleware(Some(store), None, &PlanConfig::default());
        let out = mw.before_llm_request(model_ctx("s1")).await;
        let get = |key: &str| {
            out.properties
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.as_str())
        };
        assert_eq!(get("loop.middleware.plan.mode"), Some("plan"));
        assert_eq!(get("loop.middleware.plan.id"), Some("p1"));
    }

    #[tokio::test]
    async fn plan_gate_allows_research_and_plan_tools() {
        let mw = plan_middleware(Arc::new(FakeStore::default()), PlanConfig::default()).await;
        for name in [
            "read",
            "grep",
            "find",
            "ls",
            "ask",
            "plan_write",
            "plan_read",
            "plan_list",
            "plan_submit",
            "plan_mode",
            "todo_rewrite",
            "todo_update",
        ] {
            let out = mw.before_tool(tool_ctx(name)).await;
            assert_eq!(out.control, LoopControl::Continue, "{name} must be allowed");
            assert_eq!(out.ctx.args["path"], "src/lib.rs");
        }
    }

    #[tokio::test]
    async fn execute_mode_restores_mutating_tools() {
        let store = Arc::new(FakeStore::default());
        let mw = plan_middleware(store.clone(), PlanConfig::default()).await;
        store
            .append_session_entry("s1", &PlanMode::execute().to_custom_entry_shell())
            .await
            .expect("execute mode entry");
        mw.before_llm_request(model_ctx("s1")).await;
        let out = mw.before_tool(tool_ctx("write")).await;
        assert_eq!(out.control, LoopControl::Continue);
    }

    #[tokio::test]
    async fn allowlist_extension_admits_read_only_mcp_tool() {
        let config = PlanConfig {
            allowed_tools: vec!["mcp__context7__query-docs".into()],
            ..Default::default()
        };
        let mw = plan_middleware(Arc::new(FakeStore::default()), config).await;
        let out = mw.before_tool(tool_ctx("mcp__context7__query-docs")).await;
        assert_eq!(out.control, LoopControl::Continue);
        let out = mw.before_tool(tool_ctx("mcp__context7__write")).await;
        assert_eq!(out.control, LoopControl::Stop);
    }

    #[tokio::test]
    async fn read_only_off_disables_the_gate() {
        let config = PlanConfig {
            read_only: false,
            ..Default::default()
        };
        let mw = plan_middleware(Arc::new(FakeStore::default()), config).await;
        let out = mw.before_tool(tool_ctx("write")).await;
        assert_eq!(out.control, LoopControl::Continue);
    }

    #[test]
    fn identity_and_stages_are_stable() {
        let mw = PlanModeMiddleware::new(None, None, &PlanConfig::default());
        assert_eq!(mw.id(), "plan-mode");
        assert_eq!(mw.order(), 185);
        assert_eq!(mw.stages(), PLAN_MODE_STAGES);
    }
}
