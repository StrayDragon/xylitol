use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use serde_json::Value;
use xylitol_plugin_sdk::message::AgentMessage;

pub type BeforeToolHook = Arc<dyn Fn(&str, &str, &Value) -> Option<String> + Send + Sync>;

pub type AfterToolHook =
    Arc<dyn Fn(&str, &str, Value, bool) -> Option<(Value, bool)> + Send + Sync>;

#[derive(Debug, Clone)]
pub struct ShouldStopAfterTurnCtx {
    pub turn_index: u32,
    pub assistant: Option<AgentMessage>,
    pub tool_results: Vec<AgentMessage>,
    pub history: Vec<AgentMessage>,
    pub new_messages: Vec<AgentMessage>,
}

pub type ShouldStopAfterTurnHook = Arc<dyn Fn(&ShouldStopAfterTurnCtx) -> bool + Send + Sync>;

pub fn max_turns_stop_hook(max_turns: u32) -> ShouldStopAfterTurnHook {
    let settles = AtomicU32::new(0);
    Arc::new(move |_ctx: &ShouldStopAfterTurnCtx| {
        let completed = settles.fetch_add(1, Ordering::Relaxed) + 1;
        completed >= max_turns
    })
}

#[derive(Default, Clone)]
pub struct AgentHooks {
    pub before_tool_call: Vec<BeforeToolHook>,
    pub after_tool_call: Vec<AfterToolHook>,
    pub should_stop_after_turn: Option<ShouldStopAfterTurnHook>,
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

    pub fn set_should_stop_after_turn(&mut self, hook: Option<ShouldStopAfterTurnHook>) {
        self.should_stop_after_turn = hook;
    }
}

#[cfg(test)]
mod max_turns_hook_tests {
    use super::*;

    fn ctx(turn_index: u32) -> ShouldStopAfterTurnCtx {
        ShouldStopAfterTurnCtx {
            turn_index,
            assistant: None,
            tool_results: vec![],
            history: vec![],
            new_messages: vec![],
        }
    }

    #[test]
    fn stops_when_completed_turns_reach_cap() {
        let hook = max_turns_stop_hook(2);
        assert!(!hook(&ctx(0)));
        assert!(hook(&ctx(1)));
        assert!(hook(&ctx(2)));
    }

    #[test]
    fn first_settle_does_not_use_iteration_index() {
        let hook = max_turns_stop_hook(2);
        assert!(!hook(&ctx(5)));
        assert!(hook(&ctx(0)));
    }
}
