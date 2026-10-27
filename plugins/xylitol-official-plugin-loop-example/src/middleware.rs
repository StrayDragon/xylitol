use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use xylitol_plugin_sdk::{BeforeToolCtx, DecideContinueCtx, LoopMiddleware, LoopStage, LoopStep};

use crate::config::LoopExampleConfig;

const STAGES: &[LoopStage] = &[LoopStage::BeforeTool, LoopStage::DecideContinue];

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

pub fn example_middleware(config: &LoopExampleConfig) -> Arc<dyn LoopMiddleware> {
    Arc::new(ExampleLoopMiddleware::new(config.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use xylitol_plugin_sdk::LoopControl;

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
        };
        let mw = ExampleLoopMiddleware::new(config);
        let base = DecideContinueCtx {
            turn_index: 1,
            settle_count: 1,
            has_new_user_messages: false,
            assistant: None,
            history_len: 2,
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
}
