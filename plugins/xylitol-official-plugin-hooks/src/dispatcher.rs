use std::time::Duration;

use log::{debug, warn};

use super::script::{HookScriptResult, run_hook_script, run_hook_script_with_context};
use super::{
    DispatchResult, HookAction, HookCategory, HookEvent, HookPhase, entry_matches_raw,
    event_matches, event_type_category,
};
use crate::config::{HookEntry, HooksConfig};
use xylitol_plugin_sdk::{XyHookBus, XyHookOutcome};

#[derive(Debug, Default)]
pub struct HookDispatcher {
    hooks: Vec<HookEntry>,
    default_timeout: Option<Duration>,
}

impl HookDispatcher {
    pub fn new(config: &HooksConfig) -> Self {
        let hooks = merge_hooks(config);
        let default_timeout = config.default_timeout_secs.map(Duration::from_secs);
        Self {
            hooks,
            default_timeout,
        }
    }

    pub(crate) fn entry_timeout(&self, hook: &HookEntry) -> Option<Duration> {
        hook.timeout_secs
            .map(Duration::from_secs)
            .or(self.default_timeout)
    }

    pub async fn dispatch(&self, event: &HookEvent, phase: HookPhase) -> DispatchResult {
        let category = event.category();
        let mut modified_args: Option<serde_json::Value> = None;

        for hook in &self.hooks {
            if !event_matches(hook, event, phase) {
                continue;
            }
            let result = run_hook_script(
                &hook.command,
                event,
                phase,
                self.entry_timeout(hook),
                &hook.env,
            )
            .await;
            if let Some(blocked) = self.handle_result(
                hook,
                category,
                event.event_type(),
                phase.as_str(),
                result,
                &mut modified_args,
            ) {
                return blocked;
            }
        }

        finish(modified_args)
    }

    #[allow(dead_code)]
    pub fn hook_count(&self) -> usize {
        self.hooks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.hooks.is_empty()
    }

    pub async fn dispatch_raw(
        &self,
        event_type: &str,
        phase: &str,
        context: serde_json::Value,
    ) -> DispatchResult {
        if self.hooks.is_empty() {
            return DispatchResult::Allowed;
        }

        let category = event_type_category(event_type);
        let mut modified_args: Option<serde_json::Value> = None;

        for hook in &self.hooks {
            if !entry_matches_raw(hook, event_type, phase, &context) {
                continue;
            }
            let result = run_hook_script_with_context(
                &hook.command,
                &context,
                self.entry_timeout(hook),
                &hook.env,
            )
            .await;
            if let Some(blocked) = self.handle_result(
                hook,
                category,
                event_type,
                phase,
                result,
                &mut modified_args,
            ) {
                return blocked;
            }
        }

        finish(modified_args)
    }

    fn handle_result(
        &self,
        hook: &HookEntry,
        category: HookCategory,
        event_label: &str,
        phase_label: &str,
        result: HookScriptResult,
        modified_args: &mut Option<serde_json::Value>,
    ) -> Option<DispatchResult> {
        match result {
            HookScriptResult::Action(HookAction::Allow) => {
                debug!(
                    "hook action allow event={event_label} phase={phase_label} hook={}",
                    hook.command
                );
                None
            }
            HookScriptResult::Action(HookAction::Block { reason }) => {
                warn!(
                    "Hook blocked operation event={event_label} phase={phase_label} hook={} reason={reason}",
                    hook.command
                );
                Some(DispatchResult::Blocked { reason })
            }
            HookScriptResult::Action(HookAction::Modify { args }) => {
                debug!(
                    "hook action modify event={event_label} phase={phase_label} hook={}",
                    hook.command
                );
                *modified_args = Some(args);
                None
            }
            HookScriptResult::Failed(failure) => {
                let allow = hook
                    .fail_open
                    .unwrap_or(category == HookCategory::Observation);
                let kind = failure.kind();
                if allow {
                    warn!(
                        target: "xylitol::hooks",
                        "Hook script failed, allowing (fail-open) event={event_label} phase={phase_label} hook={} error.kind={kind} reason={}",
                        hook.command,
                        failure.reason()
                    );
                    None
                } else {
                    warn!(
                        target: "xylitol::hooks",
                        "Hook script failed, blocking (fail-closed) event={event_label} phase={phase_label} hook={} error.kind={kind} reason={}",
                        hook.command,
                        failure.reason()
                    );
                    Some(DispatchResult::Blocked {
                        reason: format!(
                            "{}; migration hint: set fail_open: true on this hook to allow failures",
                            failure.reason()
                        ),
                    })
                }
            }
        }
    }
}

fn finish(modified_args: Option<serde_json::Value>) -> DispatchResult {
    match modified_args {
        Some(args) => DispatchResult::Modified { args },
        None => DispatchResult::Allowed,
    }
}

#[async_trait::async_trait]
impl XyHookBus for HookDispatcher {
    async fn dispatch(
        &self,
        event_type: &str,
        phase: &str,
        context: serde_json::Value,
    ) -> XyHookOutcome {
        match self.dispatch_raw(event_type, phase, context).await {
            DispatchResult::Allowed => XyHookOutcome::Allowed,
            DispatchResult::Blocked { reason } => XyHookOutcome::Blocked { reason },
            DispatchResult::Modified { args } => XyHookOutcome::Modified { args },
        }
    }
}

fn merge_hooks(config: &HooksConfig) -> Vec<HookEntry> {
    let mut merged: Vec<HookEntry> = Vec::new();

    let mut upsert = |entry: HookEntry| {
        let primary = entry.events.first().cloned().unwrap_or_default();
        if primary.is_empty() {
            merged.push(entry);
            return;
        }
        if let Some(pos) = merged
            .iter()
            .position(|h| h.events.first().map(|s| s.as_str()) == Some(primary.as_str()))
        {
            merged[pos] = entry;
        } else {
            merged.push(entry);
        }
    };

    for entry in &config.global {
        upsert(entry.clone());
    }
    for entry in &config.project {
        upsert(entry.clone());
    }
    for entry in &config.user {
        upsert(entry.clone());
    }

    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(events: Vec<&str>) -> HookEntry {
        HookEntry {
            events: events.into_iter().map(String::from).collect(),
            command: "echo ok".into(),
            ..Default::default()
        }
    }

    fn make_config(
        global: Vec<HookEntry>,
        project: Vec<HookEntry>,
        user: Vec<HookEntry>,
    ) -> HooksConfig {
        HooksConfig {
            global,
            project,
            user,
            default_timeout_secs: None,
        }
    }

    fn tool_call_event() -> HookEvent {
        HookEvent::ToolCall {
            tool: "bash".into(),
            args: serde_json::json!({}),
        }
    }

    fn failing_entry(fail_open: Option<bool>) -> HookEntry {
        HookEntry {
            events: vec!["pre.tool_call".into()],
            command: "exit 1".into(),
            fail_open,
            ..Default::default()
        }
    }

    #[test]
    fn test_merge_empty() {
        let config = make_config(vec![], vec![], vec![]);
        let hooks = merge_hooks(&config);
        assert!(hooks.is_empty());
    }

    #[test]
    fn test_merge_global_only() {
        let config = make_config(vec![make_entry(vec!["pre.tool_call"])], vec![], vec![]);
        let hooks = merge_hooks(&config);
        assert_eq!(hooks.len(), 1);
        assert_eq!(
            hooks[0].events.first().map(|s| s.as_str()),
            Some("pre.tool_call")
        );
    }

    #[test]
    fn test_merge_user_overrides_global() {
        let mut global_entry = make_entry(vec!["pre.tool_call"]);
        global_entry.command = "global-hook".into();
        let mut user_entry = make_entry(vec!["pre.tool_call"]);
        user_entry.command = "user-hook".into();

        let config = make_config(vec![global_entry], vec![], vec![user_entry]);
        let hooks = merge_hooks(&config);
        assert_eq!(hooks.len(), 1);
        assert_eq!(hooks[0].command, "user-hook");
    }

    #[test]
    fn test_merge_different_events_union() {
        let global = vec![make_entry(vec!["pre.tool_call"])];
        let project = vec![make_entry(vec!["post.step_complete"])];
        let config = make_config(global, project, vec![]);
        let hooks = merge_hooks(&config);
        assert_eq!(hooks.len(), 2);
    }

    #[test]
    fn test_merge_three_tier() {
        let global = vec![make_entry(vec!["tool_call"])];
        let project = vec![
            make_entry(vec!["step_complete"]),
            make_entry(vec!["model_query"]),
        ];
        let user = vec![make_entry(vec!["step_complete"])];

        let config = make_config(global, project, user);
        let hooks = merge_hooks(&config);
        assert_eq!(hooks.len(), 3);
        let sc = hooks
            .iter()
            .find(|h| h.events.first().map(|s| s.as_str()) == Some("step_complete"))
            .expect("step_complete should exist");
        assert_eq!(sc.command, "echo ok");
    }

    #[test]
    fn test_dispatcher_empty_is_noop() {
        let config = make_config(vec![], vec![], vec![]);
        let dispatcher = HookDispatcher::new(&config);
        assert!(dispatcher.is_empty());
        assert_eq!(dispatcher.hook_count(), 0);
    }

    #[test]
    fn test_dispatcher_hook_count() {
        let config = make_config(vec![make_entry(vec!["tool_call"])], vec![], vec![]);
        let dispatcher = HookDispatcher::new(&config);
        assert_eq!(dispatcher.hook_count(), 1);
        assert!(!dispatcher.is_empty());
    }

    #[tokio::test]
    async fn enforcement_failure_blocks_by_default() {
        let config = make_config(vec![failing_entry(None)], vec![], vec![]);
        let dispatcher = HookDispatcher::new(&config);
        match dispatcher
            .dispatch(&tool_call_event(), HookPhase::Pre)
            .await
        {
            DispatchResult::Blocked { reason } => {
                assert!(reason.contains("nonzero_exit"), "reason={reason}");
            }
            other => panic!("expected blocked, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn enforcement_failure_fail_open_escape_hatch() {
        let config = make_config(vec![failing_entry(Some(true))], vec![], vec![]);
        let dispatcher = HookDispatcher::new(&config);
        assert!(matches!(
            dispatcher
                .dispatch(&tool_call_event(), HookPhase::Pre)
                .await,
            DispatchResult::Allowed
        ));
    }

    #[tokio::test]
    async fn enforcement_failure_explicit_fail_closed() {
        let config = make_config(vec![failing_entry(Some(false))], vec![], vec![]);
        let dispatcher = HookDispatcher::new(&config);
        assert!(matches!(
            dispatcher
                .dispatch(&tool_call_event(), HookPhase::Pre)
                .await,
            DispatchResult::Blocked { .. }
        ));
    }

    #[tokio::test]
    async fn observation_failure_is_fail_open() {
        let mut hook = failing_entry(None);
        hook.events = vec!["step_end".into()];
        let config = make_config(vec![hook], vec![], vec![]);
        let dispatcher = HookDispatcher::new(&config);
        let event = HookEvent::StepEnd { step_index: 1 };
        assert!(matches!(
            dispatcher.dispatch(&event, HookPhase::Post).await,
            DispatchResult::Allowed
        ));
    }

    #[tokio::test]
    async fn observation_failure_can_fail_closed_via_entry() {
        let mut hook = failing_entry(Some(false));
        hook.events = vec!["step_end".into()];
        let config = make_config(vec![hook], vec![], vec![]);
        let dispatcher = HookDispatcher::new(&config);
        let event = HookEvent::StepEnd { step_index: 1 };
        assert!(matches!(
            dispatcher.dispatch(&event, HookPhase::Post).await,
            DispatchResult::Blocked { .. }
        ));
    }

    #[tokio::test]
    async fn raw_enforcement_failure_blocks() {
        let config = make_config(vec![failing_entry(None)], vec![], vec![]);
        let dispatcher = HookDispatcher::new(&config);
        match dispatcher
            .dispatch_raw("tool_call", "pre", serde_json::json!({}))
            .await
        {
            DispatchResult::Blocked { reason } => {
                assert!(reason.contains("nonzero_exit"), "reason={reason}");
            }
            other => panic!("expected blocked, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn raw_observation_failure_is_fail_open() {
        let mut hook = failing_entry(None);
        hook.events = vec!["step_end".into()];
        let config = make_config(vec![hook], vec![], vec![]);
        let dispatcher = HookDispatcher::new(&config);
        assert!(matches!(
            dispatcher
                .dispatch_raw("step_end", "post", serde_json::json!({}))
                .await,
            DispatchResult::Allowed
        ));
    }
}

#[cfg(test)]
mod timeout_tests {
    use super::*;

    fn hook_without_timeout() -> HookEntry {
        serde_json::from_value(serde_json::json!({
            "command": "echo ok",
            "events": [".*"]
        }))
        .expect("minimal hook entry")
    }

    #[test]
    fn omitted_timeout_secs_is_unlimited() {
        let hook = hook_without_timeout();
        assert!(hook.timeout_secs.is_none());
        let dispatcher = HookDispatcher::new(&HooksConfig::default());
        assert_eq!(dispatcher.entry_timeout(&hook), None);
    }

    #[test]
    fn global_default_timeout_applies() {
        let hook = hook_without_timeout();
        let config = HooksConfig {
            default_timeout_secs: Some(12),
            ..Default::default()
        };
        let dispatcher = HookDispatcher::new(&config);
        assert_eq!(
            dispatcher.entry_timeout(&hook),
            Some(Duration::from_secs(12))
        );
    }

    #[test]
    fn per_hook_timeout_overrides_global_default() {
        let mut hook = hook_without_timeout();
        hook.timeout_secs = Some(5);
        let config = HooksConfig {
            default_timeout_secs: Some(12),
            ..Default::default()
        };
        let dispatcher = HookDispatcher::new(&config);
        assert_eq!(
            dispatcher.entry_timeout(&hook),
            Some(Duration::from_secs(5))
        );
    }
}
