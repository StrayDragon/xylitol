//! Project trust resolution — determines whether a project directory is trusted.
//!
//! Aligns with pi's project-trust.ts. Resolution order:
//! 1. Explicit `trust_override` (CLI flag `--trust` / `--no-trust`)
//! 2. No trust inputs needed → auto-trust
//! 3. Extension event: ask extensions for `project_trust` decision
//! 4. TrustStore lookup (persisted decision)
//! 5. Default policy (always / never / ask)
//! 6. UI prompt (only if has_ui is true) → save to TrustStore
//! 7. Fallback: deny
//!
//! NOTE: Extension event and UI prompt integration points are stubbed for
//! future wiring. The core resolution logic is fully functional.

use crate::agent::trust::{self, TrustStore};

// ── Configuration ──────────────────────────────────────────────────

/// Default project trust policy when no explicit decision is stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DefaultProjectTrust {
    /// Always trust projects with trust inputs (auto-trust).
    Always,
    /// Never trust projects with trust inputs (auto-deny).
    Never,
    /// Prompt the user (requires UI). Falls back to deny if no UI.
    #[default]
    Ask,
}

// ── Resolution options ─────────────────────────────────────────────

/// Options for resolving project trust.
#[derive(Debug, Clone)]
pub struct ResolveTrustOptions {
    /// The project directory to check.
    pub cwd: String,
    /// The trust store for persisted decisions.
    pub trust_store: TrustStore,
    /// Explicit override from CLI (Some(true) = --trust, Some(false) = --no-trust).
    pub trust_override: Option<bool>,
    /// Default policy when no stored decision exists.
    pub default_policy: DefaultProjectTrust,
    /// Whether the current mode has UI capabilities (for prompting).
    pub has_ui: bool,
}

// ── Resolution outcome ─────────────────────────────────────────────

/// Result of trust resolution.
#[derive(Debug, Clone)]
pub struct TrustResolution {
    /// Whether the project is trusted.
    pub trusted: bool,
    /// How the decision was reached (for logging/debugging).
    pub reason: TrustReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrustReason {
    /// Explicit CLI override.
    Override,
    /// No trust inputs detected — auto-trusted.
    NoTrustInputs,
    /// Resolved via extension event.
    Extension,
    /// Found in persistent store.
    Store,
    /// Resolved via default policy.
    DefaultPolicy,
    /// User chose via UI prompt.
    UserPrompt,
    /// Fallback: no UI available, default is Ask → deny.
    FallbackNoUi,
}

// ── Resolution logic ───────────────────────────────────────────────

/// Resolve whether a project directory should be trusted.
///
/// # Arguments
/// * `options` — resolution options (CWD, store, override, policy, has_ui)
/// * `on_prompt` — callback for UI prompt. Receives a list of trust options
///   and should return the selected option index (or None to cancel/deny).
///   Only called when `has_ui` is true and `default_policy` is `Ask`.
pub fn resolve_project_trusted<F>(options: &ResolveTrustOptions, on_prompt: F) -> TrustResolution
where
    F: FnOnce(&[trust::TrustOption]) -> Option<usize>,
{
    // 1. Explicit override
    if let Some(overridden) = options.trust_override {
        return TrustResolution {
            trusted: overridden,
            reason: TrustReason::Override,
        };
    }

    // 2. No trust inputs → auto-trust
    if !trust::has_project_trust_inputs(&options.cwd) {
        return TrustResolution {
            trusted: true,
            reason: TrustReason::NoTrustInputs,
        };
    }

    // 3. Extension event — stubbed for future extension wire-up
    // TODO: Call emit_project_trust_event when extension system is integrated.

    // 4. TrustStore lookup
    let store_decision = options.trust_store.get(&options.cwd);
    if let Some(decision) = store_decision {
        return TrustResolution {
            trusted: decision,
            reason: TrustReason::Store,
        };
    }

    // 5. Default policy
    match options.default_policy {
        DefaultProjectTrust::Always => {
            return TrustResolution {
                trusted: true,
                reason: TrustReason::DefaultPolicy,
            };
        }
        DefaultProjectTrust::Never => {
            return TrustResolution {
                trusted: false,
                reason: TrustReason::DefaultPolicy,
            };
        }
        DefaultProjectTrust::Ask => { /* continue to UI */ }
    }

    // 6. UI prompt (only if has_ui)
    if !options.has_ui {
        return TrustResolution {
            trusted: false,
            reason: TrustReason::FallbackNoUi,
        };
    }

    let trust_options = trust::get_project_trust_options(&options.cwd);
    if let Some(selected_idx) = on_prompt(&trust_options)
        && let Some(selected) = trust_options.get(selected_idx)
    {
        // Persist the decision
        if !selected.updates.is_empty() {
            options.trust_store.set_many(&selected.updates);
        }
        return TrustResolution {
            trusted: selected.trusted,
            reason: TrustReason::UserPrompt,
        };
    }

    // 7. Fallback: prompt cancelled or invalid response → deny
    TrustResolution {
        trusted: false,
        reason: TrustReason::FallbackNoUi,
    }
}

// ── Trust prompt formatting ─────────────────────────────────────────

/// Format the trust prompt message for display to the user.
pub fn format_trust_prompt(cwd: &str) -> String {
    format!(
        "Trust project folder?\n{cwd}\n\n\
         This allows xylitol to load .xylitol settings and resources, \
         and execute project extensions."
    )
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_resolve_override_trusted() {
        let tmp = tempfile::tempdir().unwrap();
        let store = TrustStore::new(tmp.path());
        let cwd = std::env::current_dir()
            .unwrap()
            .to_string_lossy()
            .to_string();

        let opts = ResolveTrustOptions {
            cwd: cwd.clone(),
            trust_store: store,
            trust_override: Some(true),
            default_policy: DefaultProjectTrust::Ask,
            has_ui: true,
        };

        let result = resolve_project_trusted(&opts, |_| None);
        assert!(result.trusted);
        assert_eq!(result.reason, TrustReason::Override);
    }

    #[test]
    fn test_resolve_override_not_trusted() {
        let tmp = tempfile::tempdir().unwrap();
        let store = TrustStore::new(tmp.path());
        let cwd = std::env::current_dir()
            .unwrap()
            .to_string_lossy()
            .to_string();

        let opts = ResolveTrustOptions {
            cwd: cwd.clone(),
            trust_store: store,
            trust_override: Some(false),
            default_policy: DefaultProjectTrust::Ask,
            has_ui: true,
        };

        let result = resolve_project_trusted(&opts, |_| None);
        assert!(!result.trusted);
        assert_eq!(result.reason, TrustReason::Override);
    }

    #[test]
    fn test_resolve_no_inputs_auto_trust() {
        let tmp = tempfile::tempdir().unwrap();
        let store = TrustStore::new(tmp.path());
        // Empty temp dir — no .xylitol/ or .agents/skills/
        let cwd = tmp.path().to_string_lossy().to_string();

        let opts = ResolveTrustOptions {
            cwd,
            trust_store: store,
            trust_override: None,
            default_policy: DefaultProjectTrust::Ask,
            has_ui: true,
        };

        let result = resolve_project_trusted(&opts, |_| None);
        assert!(result.trusted);
        assert_eq!(result.reason, TrustReason::NoTrustInputs);
    }

    #[test]
    fn test_resolve_stored_decision() {
        let tmp = tempfile::tempdir().unwrap();
        // Create trust inputs
        fs::create_dir_all(tmp.path().join(".xylitol")).unwrap();
        let cwd = tmp.path().to_string_lossy().to_string();

        let store = TrustStore::new(tmp.path());
        store.set(&cwd, Some(false));

        let opts = ResolveTrustOptions {
            cwd: cwd.clone(),
            trust_store: store,
            trust_override: None,
            default_policy: DefaultProjectTrust::Ask,
            has_ui: true,
        };

        let result = resolve_project_trusted(&opts, |_| None);
        assert!(!result.trusted);
        assert_eq!(result.reason, TrustReason::Store);
    }

    #[test]
    fn test_resolve_default_always() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join(".xylitol")).unwrap();
        let cwd = tmp.path().to_string_lossy().to_string();

        let store = TrustStore::new(tmp.path());

        let opts = ResolveTrustOptions {
            cwd,
            trust_store: store,
            trust_override: None,
            default_policy: DefaultProjectTrust::Always,
            has_ui: true,
        };

        let result = resolve_project_trusted(&opts, |_| None);
        assert!(result.trusted);
        assert_eq!(result.reason, TrustReason::DefaultPolicy);
    }

    #[test]
    fn test_resolve_default_never() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join(".xylitol")).unwrap();
        let cwd = tmp.path().to_string_lossy().to_string();

        let store = TrustStore::new(tmp.path());

        let opts = ResolveTrustOptions {
            cwd,
            trust_store: store,
            trust_override: None,
            default_policy: DefaultProjectTrust::Never,
            has_ui: true,
        };

        let result = resolve_project_trusted(&opts, |_| None);
        assert!(!result.trusted);
        assert_eq!(result.reason, TrustReason::DefaultPolicy);
    }

    #[test]
    fn test_resolve_no_ui_fallback() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join(".xylitol")).unwrap();
        let cwd = tmp.path().to_string_lossy().to_string();

        let store = TrustStore::new(tmp.path());

        let opts = ResolveTrustOptions {
            cwd,
            trust_store: store,
            trust_override: None,
            default_policy: DefaultProjectTrust::Ask,
            has_ui: false,
        };

        let result = resolve_project_trusted(&opts, |_| None);
        assert!(!result.trusted);
        assert_eq!(result.reason, TrustReason::FallbackNoUi);
    }

    #[test]
    fn test_resolve_user_prompt_choose_trust() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join(".xylitol")).unwrap();
        let cwd = tmp.path().to_string_lossy().to_string();

        let store = TrustStore::new(tmp.path());

        let opts = ResolveTrustOptions {
            cwd: cwd.clone(),
            trust_store: store.clone(),
            trust_override: None,
            default_policy: DefaultProjectTrust::Ask,
            has_ui: true,
        };

        // User selects "Trust" (index 0)
        let result = resolve_project_trusted(&opts, |options| {
            assert!(!options.is_empty());
            Some(0)
        });
        assert!(result.trusted);
        assert_eq!(result.reason, TrustReason::UserPrompt);

        // The decision should now be persisted
        assert_eq!(store.get(&cwd), Some(true));
    }

    #[test]
    fn test_resolve_user_prompt_cancelled() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join(".xylitol")).unwrap();
        let cwd = tmp.path().to_string_lossy().to_string();

        let store = TrustStore::new(tmp.path());

        let opts = ResolveTrustOptions {
            cwd,
            trust_store: store,
            trust_override: None,
            default_policy: DefaultProjectTrust::Ask,
            has_ui: true,
        };

        // User cancels (None)
        let result = resolve_project_trusted(&opts, |_| None);
        assert!(!result.trusted);
        assert_eq!(result.reason, TrustReason::FallbackNoUi);
    }

    #[test]
    fn test_format_trust_prompt_contains_info() {
        let msg = format_trust_prompt("/home/user/project");
        assert!(msg.contains("Trust project folder?"));
        assert!(msg.contains("/home/user/project"));
        assert!(msg.contains(".xylitol"));
    }
}
