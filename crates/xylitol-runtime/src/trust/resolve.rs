//! Project trust resolution — fixed-precedence pipeline (spec c255 / t3, t4).
//!
//! Resolution order:
//! 1. Explicit `trust_override` (CLI flag `--trust` / `--no-trust`)
//! 2. No trust inputs detected → auto-trust
//! 3. Persistent store lookup (with parent inheritance)
//! 4. Configured default policy (always / never / ask)
//! 5. Interactive UI prompt (only when `has_ui`), via the injected `on_prompt`
//!    callback — this module performs no terminal I/O (spec t4)
//! 6. Fallback: deny
//!
//! The resolver returns both the decision and the reason that produced it.

use super::store::{TrustManager, TrustOption};

// ── Configuration ──────────────────────────────────────────────────

/// Default project trust policy when no explicit decision is stored.
///
/// Only `Ask` ships (product default; auto-trust / auto-deny policy knobs
/// removed in c2750 — no config path constructs them).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DefaultProjectTrust {
    /// Prompt the user (requires UI). Falls back to deny if no UI.
    #[default]
    Ask,
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

/// How a trust decision was reached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrustReason {
    /// Explicit CLI override.
    Override,
    /// No trust inputs detected — auto-trusted.
    NoTrustInputs,
    /// Found in persistent store.
    Store,
    /// User chose via UI prompt.
    UserPrompt,
    /// Fallback: no UI available, default is Ask → deny.
    FallbackNoUi,
}

// ── Resolution logic ───────────────────────────────────────────────

/// Resolve whether a project directory should be trusted.
///
/// # Arguments
/// * `manager` — the trust store (single source of truth)
/// * `cwd` — the project directory to check
/// * `trust_override` — explicit CLI override (`Some(true)` = `--trust`,
///   `Some(false)` = `--no-trust`)
/// * `default_policy` — default policy when no stored decision exists
/// * `has_ui` — whether the current mode can prompt the user
/// * `on_prompt` — callback for the UI prompt. Receives the trust options and
///   should return the selected option index (or `None` to cancel/deny). Only
///   called when `has_ui` is true and `default_policy` is `Ask` (spec t4).
pub fn resolve_project_trusted<F>(
    manager: &TrustManager,
    cwd: &str,
    trust_override: Option<bool>,
    _default_policy: DefaultProjectTrust,
    has_ui: bool,
    on_prompt: F,
) -> TrustResolution
where
    F: FnOnce(&[TrustOption]) -> Option<usize>,
{
    // 1. Explicit override
    if let Some(overridden) = trust_override {
        return TrustResolution {
            trusted: overridden,
            reason: TrustReason::Override,
        };
    }

    // 2. No trust inputs → auto-trust
    if !manager.has_trust_inputs(cwd) {
        return TrustResolution {
            trusted: true,
            reason: TrustReason::NoTrustInputs,
        };
    }

    // 3. Store lookup (with inheritance)
    if let Some(decision) = manager.is_project_trusted(cwd) {
        return TrustResolution {
            trusted: decision,
            reason: TrustReason::Store,
        };
    }

    // 4. Default policy (single `Ask` layout — Always/Never removed in c2750)

    // 5. UI prompt (only if has_ui)
    if !has_ui {
        return TrustResolution {
            trusted: false,
            reason: TrustReason::FallbackNoUi,
        };
    }

    let trust_options = manager.get_trust_options(cwd, false);
    if let Some(selected_idx) = on_prompt(&trust_options)
        && let Some(selected) = trust_options.get(selected_idx)
    {
        // Persist the decision (session-only options carry no updates).
        if !selected.updates.is_empty() {
            let _ = manager.apply_updates(&selected.updates);
        }
        return TrustResolution {
            trusted: selected.trusted,
            reason: TrustReason::UserPrompt,
        };
    }

    // 6. Fallback: prompt cancelled or invalid response → deny
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

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn store() -> (TrustManager, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let mgr = TrustManager::new(dir.path().to_path_buf());
        (mgr, dir)
    }

    #[test]
    fn test_resolve_override_trusted() {
        let (mgr, dir) = store();
        fs::create_dir_all(dir.path().join(".xylitol")).unwrap();
        let cwd = dir.path().to_string_lossy().to_string();
        let result =
            resolve_project_trusted(&mgr, &cwd, Some(true), Default::default(), true, |_| None);
        assert!(result.trusted);
        assert_eq!(result.reason, TrustReason::Override);
    }

    #[test]
    fn test_resolve_override_not_trusted() {
        let (mgr, dir) = store();
        fs::create_dir_all(dir.path().join(".xylitol")).unwrap();
        let cwd = dir.path().to_string_lossy().to_string();
        let result =
            resolve_project_trusted(&mgr, &cwd, Some(false), Default::default(), true, |_| None);
        assert!(!result.trusted);
        assert_eq!(result.reason, TrustReason::Override);
    }

    #[test]
    fn test_resolve_no_inputs_auto_trust() {
        let (mgr, dir) = store();
        // Empty temp dir — no .xylitol/ or .agents/skills/.
        let cwd = dir.path().to_string_lossy().to_string();
        let result = resolve_project_trusted(&mgr, &cwd, None, Default::default(), true, |_| None);
        assert!(result.trusted);
        assert_eq!(result.reason, TrustReason::NoTrustInputs);
    }

    #[test]
    fn test_resolve_stored_decision() {
        let (mgr, dir) = store();
        fs::create_dir_all(dir.path().join(".xylitol")).unwrap();
        let cwd = dir.path().to_string_lossy().to_string();
        mgr.set_trust(&cwd, Some(false)).unwrap();

        let result = resolve_project_trusted(&mgr, &cwd, None, Default::default(), true, |_| None);
        assert!(!result.trusted);
        assert_eq!(result.reason, TrustReason::Store);
    }

    #[test]
    fn test_resolve_no_ui_fallback() {
        let (mgr, dir) = store();
        fs::create_dir_all(dir.path().join(".xylitol")).unwrap();
        let cwd = dir.path().to_string_lossy().to_string();

        let result =
            resolve_project_trusted(&mgr, &cwd, None, DefaultProjectTrust::Ask, false, |_| None);
        assert!(!result.trusted);
        assert_eq!(result.reason, TrustReason::FallbackNoUi);
    }

    #[test]
    fn test_resolve_user_prompt_choose_trust() {
        let (mgr, dir) = store();
        fs::create_dir_all(dir.path().join(".xylitol")).unwrap();
        let cwd = dir.path().to_string_lossy().to_string();

        // User selects "Trust" (index 0).
        let result = resolve_project_trusted(
            &mgr,
            &cwd,
            None,
            DefaultProjectTrust::Ask,
            true,
            |options| {
                assert!(!options.is_empty());
                Some(0)
            },
        );
        assert!(result.trusted);
        assert_eq!(result.reason, TrustReason::UserPrompt);

        // The decision should now be persisted.
        assert_eq!(mgr.is_project_trusted(&cwd), Some(true));
    }

    #[test]
    fn test_resolve_user_prompt_cancelled() {
        let (mgr, dir) = store();
        fs::create_dir_all(dir.path().join(".xylitol")).unwrap();
        let cwd = dir.path().to_string_lossy().to_string();

        let result =
            resolve_project_trusted(&mgr, &cwd, None, DefaultProjectTrust::Ask, true, |_| None);
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
