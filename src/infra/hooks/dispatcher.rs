//! HookDispatcher — event dispatch with serial execution and timeout control.

use std::time::Duration;

use log::warn;

use super::script::{run_hook_script, run_hook_script_with_context};
use super::{DispatchResult, HookAction, HookEvent, HookPhase, entry_matches_raw, event_matches};
use crate::infra::config::types::{HookEntry, HooksConfig};
use crate::runtime_protocol::{XyHookBus, XyHookOutcome};

/// Default timeout per hook script execution.
const DEFAULT_TIMEOUT_SECS: u64 = 5;

/// Hook event dispatcher.
///
/// Manages a merged list of hooks from three tiers (global/project/user) and
/// provides [`dispatch`](HookDispatcher::dispatch) to execute matching hooks
/// serially. A hook that returns `block` immediately terminates the chain.
///
/// When no hooks are configured, dispatch is a no-op returning `Allowed`.
#[derive(Debug, Default)]
pub struct HookDispatcher {
    /// Merged hook entries (user overrides project overrides global).
    hooks: Vec<HookEntry>,
    /// Timeout for hook execution (from config, applied per-invoke).
    #[allow(dead_code)]
    timeout: Duration,
}

impl HookDispatcher {
    /// Create a new dispatcher from the three-tier config.
    ///
    /// Merges hooks by event pattern: user > project > global.
    pub fn new(config: &HooksConfig) -> Self {
        let hooks = merge_hooks(config);
        Self {
            hooks,
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
        }
    }

    /// Dispatch an event to all matching hooks.
    ///
    /// Hooks are executed in order: global → project → user. If any hook
    /// returns `Block`, the chain terminates immediately. A `Modify` result
    /// updates the args for subsequent hooks.
    pub async fn dispatch(&self, event: &HookEvent, phase: HookPhase) -> DispatchResult {
        let mut modified_args: Option<serde_json::Value> = None;

        for hook in &self.hooks {
            if !event_matches(hook, event, phase) {
                continue;
            }

            let timeout = Duration::from_secs(hook.timeout_secs.max(1));

            let result = run_hook_script(&hook.command, event, phase, timeout, &hook.env).await;

            match result {
                HookAction::Allow => {
                    // Continue to next hook.
                }
                HookAction::Block { reason } => {
                    warn!(
                        "Hook blocked operation event={} phase={} hook={} reason={}",
                        event.event_type(),
                        phase.as_str(),
                        hook.command,
                        reason
                    );
                    return DispatchResult::Blocked { reason };
                }
                HookAction::Modify { args } => {
                    modified_args = Some(args);
                }
            }
        }

        if let Some(args) = modified_args {
            DispatchResult::Modified { args }
        } else {
            DispatchResult::Allowed
        }
    }

    /// Number of registered hooks.
    pub fn hook_count(&self) -> usize {
        self.hooks.len()
    }

    /// Whether any hooks are registered.
    pub fn is_empty(&self) -> bool {
        self.hooks.is_empty()
    }

    /// Dispatch using pi-aligned event type strings and a JSON context.
    ///
    /// When no hooks are registered this is a zero-overhead no-op returning
    /// [`DispatchResult::Allowed`].
    pub async fn dispatch_raw(
        &self,
        event_type: &str,
        phase: &str,
        context: serde_json::Value,
    ) -> DispatchResult {
        if self.hooks.is_empty() {
            return DispatchResult::Allowed;
        }

        let mut modified_args: Option<serde_json::Value> = None;

        for hook in &self.hooks {
            if !entry_matches_raw(hook, event_type, phase, &context) {
                continue;
            }

            let timeout = Duration::from_secs(hook.timeout_secs.max(1));

            let result =
                run_hook_script_with_context(&hook.command, &context, timeout, &hook.env).await;

            match result {
                HookAction::Allow => {}
                HookAction::Block { reason } => {
                    warn!(
                        "Hook blocked operation event={} phase={} hook={} reason={}",
                        event_type, phase, hook.command, reason
                    );
                    return DispatchResult::Blocked { reason };
                }
                HookAction::Modify { args } => {
                    modified_args = Some(args);
                }
            }
        }

        if let Some(args) = modified_args {
            DispatchResult::Modified { args }
        } else {
            DispatchResult::Allowed
        }
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

/// Merge three-tier hooks into a single list.
///
/// Rules:
/// - Hooks with the same primary event pattern (first in `events`) are dedup'd:
///   user overrides project, project overrides global.
/// - Non-overlapping event patterns are unioned.
fn merge_hooks(config: &HooksConfig) -> Vec<HookEntry> {
    // Collect all hooks keyed by primary event pattern.
    let mut merged: Vec<HookEntry> = Vec::new();

    // Helper: insert or replace by primary event pattern.
    let mut upsert = |entry: HookEntry| {
        let primary = entry.events.first().cloned().unwrap_or_default();
        if primary.is_empty() {
            // No event pattern — always append.
            merged.push(entry);
            return;
        }
        if let Some(pos) = merged
            .iter()
            .position(|h| h.events.first().map(|s| s.as_str()) == Some(primary.as_str()))
        {
            // Replace existing entry (later tier wins).
            merged[pos] = entry;
        } else {
            merged.push(entry);
        }
    };

    // Apply in order: global → project → user.
    // Later calls overwrite earlier ones with same primary event pattern.
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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
        let user = vec![make_entry(vec!["step_complete"])]; // overrides project

        let config = make_config(global, project, user);
        let hooks = merge_hooks(&config);
        // tool_call (global), step_complete (user→overrides), model_query (project)
        assert_eq!(hooks.len(), 3);
        // step_complete should be the user entry (which uses defaults)
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
}
