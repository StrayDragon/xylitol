use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use xylitol_plugin_sdk::message::{AgentMessage, XyLlmMessage};
use xylitol_plugin_sdk::session::{PlanMode, PlanModeKind};
use xylitol_plugin_sdk::{BeforeLlmRequestCtx, BeforeToolCtx, LoopMiddleware, LoopStage, LoopStep};

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
    mode: Mutex<PlanMode>,
    read_only: bool,
    allowed_tools: Vec<String>,
}

impl PlanModeMiddleware {
    pub fn new(config: &PlanConfig) -> Self {
        Self {
            mode: Mutex::new(PlanMode::execute()),
            read_only: config.read_only,
            allowed_tools: config.allowed_tools.clone(),
        }
    }

    fn allows(&self, tool_name: &str) -> bool {
        READ_ONLY_ALLOWLIST.contains(&tool_name)
            || self.allowed_tools.iter().any(|tool| tool == tool_name)
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
        let mode = ctx.plan_mode.clone().unwrap_or_default();
        *self.mode.lock().unwrap_or_else(|e| e.into_inner()) = mode.clone();
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
        ctx.deny_reason = Some("plan mode is read-only".into());
        LoopStep::continued(ctx)
            .with_property("loop.middleware.plan.read_only", "true")
            .with_property("loop.middleware.plan.denied_tool", denied_tool)
    }
}

pub fn plan_mode_middleware(config: &PlanConfig) -> Arc<dyn LoopMiddleware> {
    Arc::new(PlanModeMiddleware::new(config))
}

#[cfg(test)]
mod tests {
    use super::*;
    use xylitol_plugin_sdk::LoopControl;

    fn model_ctx(session_id: &str, plan_mode: Option<PlanMode>) -> BeforeLlmRequestCtx {
        BeforeLlmRequestCtx {
            turn_index: 0,
            messages: Vec::new(),
            tool_schemas: Vec::new(),
            options: xylitol_plugin_sdk::XyGenerateOptions::default(),
            todo: None,
            plan_mode,
            session_id: Some(session_id.into()),
        }
    }

    fn tool_ctx(name: &str) -> BeforeToolCtx {
        BeforeToolCtx {
            turn_index: 0,
            tool_call_id: "call-1".into(),
            tool_name: name.into(),
            args: serde_json::json!({ "path": "src/lib.rs" }),
            deny_reason: None,
        }
    }

    async fn plan_middleware(config: PlanConfig, plan_mode: PlanMode) -> Arc<dyn LoopMiddleware> {
        let mw = plan_mode_middleware(&config);
        mw.before_llm_request(model_ctx("s1", Some(plan_mode)))
            .await;
        mw
    }

    #[tokio::test]
    async fn plan_gate_denies_mutating_and_unknown_tools() {
        let mw = plan_middleware(PlanConfig::default(), PlanMode::plan(None)).await;
        for name in ["write", "edit", "bash", "mcp__write", "unknown_tool"] {
            let out = mw.before_tool(tool_ctx(name)).await;
            assert_eq!(
                out.control,
                LoopControl::Continue,
                "{name} must be denied without ending the run"
            );
            assert_eq!(
                out.ctx.deny_reason.as_deref(),
                Some("plan mode is read-only")
            );
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
        let mw = plan_mode_middleware(&PlanConfig::default());
        let out = mw
            .before_llm_request(model_ctx("s1", Some(PlanMode::plan(Some("p1".into())))))
            .await;
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
    async fn missing_plan_mode_degrades_to_execute() {
        let mw = plan_mode_middleware(&PlanConfig::default());
        let out = mw.before_llm_request(model_ctx("s1", None)).await;
        assert_eq!(out.control, LoopControl::Continue);
        assert!(
            !out.properties
                .iter()
                .any(|(key, _)| key == "loop.middleware.plan.mode"),
            "no plan mode must not claim plan"
        );
        let denied = mw.before_tool(tool_ctx("write")).await;
        assert_eq!(denied.control, LoopControl::Continue);
    }

    #[tokio::test]
    async fn plan_gate_allows_research_and_plan_tools() {
        let mw = plan_middleware(PlanConfig::default(), PlanMode::plan(None)).await;
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
        let mw = plan_middleware(PlanConfig::default(), PlanMode::execute()).await;
        let out = mw.before_tool(tool_ctx("write")).await;
        assert_eq!(out.control, LoopControl::Continue);
    }

    #[tokio::test]
    async fn allowlist_extension_admits_read_only_mcp_tool() {
        let config = PlanConfig {
            allowed_tools: vec!["mcp__context7__query-docs".into()],
            ..Default::default()
        };
        let mw = plan_middleware(config, PlanMode::plan(None)).await;
        let out = mw.before_tool(tool_ctx("mcp__context7__query-docs")).await;
        assert_eq!(out.control, LoopControl::Continue);
        let out = mw.before_tool(tool_ctx("mcp__context7__write")).await;
        assert_eq!(out.control, LoopControl::Continue);
        assert_eq!(
            out.ctx.deny_reason.as_deref(),
            Some("plan mode is read-only")
        );
    }

    #[tokio::test]
    async fn read_only_off_disables_the_gate() {
        let config = PlanConfig {
            read_only: false,
            ..Default::default()
        };
        let mw = plan_middleware(config, PlanMode::plan(None)).await;
        let out = mw.before_tool(tool_ctx("write")).await;
        assert_eq!(out.control, LoopControl::Continue);
    }

    #[test]
    fn identity_and_stages_are_stable() {
        let mw = PlanModeMiddleware::new(&PlanConfig::default());
        assert_eq!(mw.id(), "plan-mode");
        assert_eq!(mw.order(), 185);
        assert_eq!(mw.stages(), PLAN_MODE_STAGES);
    }
}
