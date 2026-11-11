use std::sync::Arc;

use serde_json::Value;
use xylitol_plugin_sdk::message::AgentMessage;

pub type BeforeToolHook = Arc<dyn Fn(&str, &str, &Value) -> Option<String> + Send + Sync>;

pub type AfterToolHook =
    Arc<dyn Fn(&str, &str, Value, bool) -> Option<(Value, bool)> + Send + Sync>;

#[derive(Debug, Clone)]
pub struct ShouldStopAfterStepCtx {
    pub step_index: u32,
    pub is_settle: bool,
    pub assistant: Option<AgentMessage>,
    pub tool_results: Vec<AgentMessage>,
    pub history: Vec<AgentMessage>,
    pub new_messages: Vec<AgentMessage>,
}

pub type ShouldStopAfterStepHook = Arc<dyn Fn(&ShouldStopAfterStepCtx) -> bool + Send + Sync>;

pub fn max_steps_stop_hook(max_steps_per_turn: u32) -> ShouldStopAfterStepHook {
    Arc::new(move |ctx: &ShouldStopAfterStepCtx| {
        ctx.step_index.saturating_add(1) >= max_steps_per_turn
    })
}

#[derive(Default, Clone)]
pub struct AgentHooks {
    pub before_tool_call: Vec<BeforeToolHook>,
    pub after_tool_call: Vec<AfterToolHook>,
    pub should_stop_after_step: Option<ShouldStopAfterStepHook>,
}

impl AgentHooks {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn add_before(&mut self, hook: BeforeToolHook) {
        self.before_tool_call.push(hook);
    }

    pub fn add_after(&mut self, hook: AfterToolHook) {
        self.after_tool_call.push(hook);
    }

    pub fn set_should_stop_after_step(&mut self, hook: Option<ShouldStopAfterStepHook>) {
        self.should_stop_after_step = hook;
    }
}

#[cfg(test)]
mod max_steps_hook_tests {
    use super::*;

    fn ctx(step_index: u32, is_settle: bool) -> ShouldStopAfterStepCtx {
        ShouldStopAfterStepCtx {
            step_index,
            is_settle,
            assistant: None,
            tool_results: vec![],
            history: vec![],
            new_messages: vec![],
        }
    }

    #[test]
    fn stops_when_step_index_reaches_cap() {
        let hook = max_steps_stop_hook(2);
        assert!(!hook(&ctx(0, false)));
        assert!(hook(&ctx(1, true)));
        assert!(hook(&ctx(2, true)));
    }

    #[test]
    fn single_step_cap_stops_on_first_step() {
        let hook = max_steps_stop_hook(1);
        assert!(hook(&ctx(0, false)));
    }

    #[test]
    fn continue_tools_step_counts_against_cap() {
        let hook = max_steps_stop_hook(2);
        assert!(hook(&ctx(1, false)));
    }

    #[test]
    fn unconfigured_hooks_never_stop() {
        assert!(AgentHooks::default().should_stop_after_step.is_none());
    }
}
