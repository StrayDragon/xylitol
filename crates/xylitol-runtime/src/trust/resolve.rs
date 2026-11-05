use super::store::{TrustAuditContext, TrustManager, TrustOption};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DefaultProjectTrust {
    #[default]
    Ask,
}

#[derive(Debug, Clone)]
pub struct TrustResolution {
    pub trusted: bool,
    pub reason: TrustReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrustReason {
    Override,
    NoTrustInputs,
    Store,
    UserPrompt,
    FallbackNoUi,
}

pub fn resolve_project_trusted<F>(
    manager: &TrustManager,
    cwd: &str,
    trust_override: Option<bool>,
    _default_policy: DefaultProjectTrust,
    has_ui: bool,
    audit: &TrustAuditContext,
    on_prompt: F,
) -> TrustResolution
where
    F: FnOnce(&[TrustOption]) -> Option<usize>,
{
    if let Some(overridden) = trust_override {
        let event = if overridden {
            "trust_grant"
        } else {
            "trust_deny"
        };
        manager.record_decision(event, cwd, Some(overridden), audit, "override");
        return TrustResolution {
            trusted: overridden,
            reason: TrustReason::Override,
        };
    }

    if !manager.has_trust_inputs(cwd) {
        manager.record_decision("trust_grant", cwd, Some(true), audit, "no_trust_inputs");
        return TrustResolution {
            trusted: true,
            reason: TrustReason::NoTrustInputs,
        };
    }

    if let Some(decision) = manager.is_project_trusted(cwd) {
        return TrustResolution {
            trusted: decision,
            reason: TrustReason::Store,
        };
    }

    if !has_ui {
        manager.record_decision("trust_deny", cwd, None, audit, "no_ui");
        return TrustResolution {
            trusted: false,
            reason: TrustReason::FallbackNoUi,
        };
    }

    let trust_options = manager.get_trust_options(cwd, false);
    if let Some(selected_idx) = on_prompt(&trust_options)
        && let Some(selected) = trust_options.get(selected_idx)
    {
        if !selected.updates.is_empty() {
            let _ = manager.apply_updates(&selected.updates, audit);
        } else {
            let event = if selected.trusted {
                "trust_grant"
            } else {
                "trust_deny"
            };
            manager.record_decision(event, cwd, Some(selected.trusted), audit, "session_only");
        }
        return TrustResolution {
            trusted: selected.trusted,
            reason: TrustReason::UserPrompt,
        };
    }

    manager.record_decision("trust_deny", cwd, None, audit, "prompt_cancelled");
    TrustResolution {
        trusted: false,
        reason: TrustReason::FallbackNoUi,
    }
}

pub fn format_trust_prompt(cwd: &str) -> String {
    format!(
        "Trust project folder?\n{cwd}\n\n\
         This allows xylitol to load .xylitol settings and resources, \
         and execute project extensions."
    )
}

#[cfg(test)]
mod tests {
    use super::super::store::{TrustActor, TrustSource};
    use super::*;
    use std::fs;

    fn store() -> (TrustManager, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let mgr = TrustManager::new(dir.path().to_path_buf());
        (mgr, dir)
    }

    fn ctx() -> TrustAuditContext {
        TrustAuditContext::new(TrustActor::Cli, TrustSource::Startup)
    }

    #[test]
    fn test_resolve_override_trusted() {
        let (mgr, dir) = store();
        fs::create_dir_all(dir.path().join(".xylitol")).unwrap();
        let cwd = dir.path().to_string_lossy().to_string();
        let result = resolve_project_trusted(
            &mgr,
            &cwd,
            Some(true),
            Default::default(),
            true,
            &ctx(),
            |_| None,
        );
        assert!(result.trusted);
        assert_eq!(result.reason, TrustReason::Override);
    }

    #[test]
    fn test_resolve_override_not_trusted() {
        let (mgr, dir) = store();
        fs::create_dir_all(dir.path().join(".xylitol")).unwrap();
        let cwd = dir.path().to_string_lossy().to_string();
        let result = resolve_project_trusted(
            &mgr,
            &cwd,
            Some(false),
            Default::default(),
            true,
            &ctx(),
            |_| None,
        );
        assert!(!result.trusted);
        assert_eq!(result.reason, TrustReason::Override);
    }

    #[test]
    fn test_resolve_no_inputs_auto_trust() {
        let (mgr, dir) = store();
        let cwd = dir.path().to_string_lossy().to_string();
        let result =
            resolve_project_trusted(&mgr, &cwd, None, Default::default(), true, &ctx(), |_| None);
        assert!(result.trusted);
        assert_eq!(result.reason, TrustReason::NoTrustInputs);
    }

    #[test]
    fn test_resolve_stored_decision() {
        let (mgr, dir) = store();
        fs::create_dir_all(dir.path().join(".xylitol")).unwrap();
        let cwd = dir.path().to_string_lossy().to_string();
        mgr.apply_updates(
            &[super::super::store::TrustUpdate {
                path: cwd.clone(),
                decision: Some(false),
            }],
            &ctx(),
        )
        .unwrap();

        let result =
            resolve_project_trusted(&mgr, &cwd, None, Default::default(), true, &ctx(), |_| None);
        assert!(!result.trusted);
        assert_eq!(result.reason, TrustReason::Store);
    }

    #[test]
    fn test_resolve_no_ui_fallback() {
        let (mgr, dir) = store();
        fs::create_dir_all(dir.path().join(".xylitol")).unwrap();
        let cwd = dir.path().to_string_lossy().to_string();

        let result = resolve_project_trusted(
            &mgr,
            &cwd,
            None,
            DefaultProjectTrust::Ask,
            false,
            &ctx(),
            |_| None,
        );
        assert!(!result.trusted);
        assert_eq!(result.reason, TrustReason::FallbackNoUi);
    }

    #[test]
    fn test_resolve_user_prompt_choose_trust() {
        let (mgr, dir) = store();
        fs::create_dir_all(dir.path().join(".xylitol")).unwrap();
        let cwd = dir.path().to_string_lossy().to_string();

        let result = resolve_project_trusted(
            &mgr,
            &cwd,
            None,
            DefaultProjectTrust::Ask,
            true,
            &ctx(),
            |options| {
                assert!(!options.is_empty());
                Some(0)
            },
        );
        assert!(result.trusted);
        assert_eq!(result.reason, TrustReason::UserPrompt);

        assert_eq!(mgr.is_project_trusted(&cwd), Some(true));
    }

    #[test]
    fn test_resolve_user_prompt_cancelled() {
        let (mgr, dir) = store();
        fs::create_dir_all(dir.path().join(".xylitol")).unwrap();
        let cwd = dir.path().to_string_lossy().to_string();

        let result = resolve_project_trusted(
            &mgr,
            &cwd,
            None,
            DefaultProjectTrust::Ask,
            true,
            &ctx(),
            |_| None,
        );
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
