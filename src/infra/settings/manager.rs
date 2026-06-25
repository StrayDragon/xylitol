//! SettingsManager — three-tier deep merge, reload, lock-based persistence.

use std::sync::{Arc, Mutex};

use super::storage::{
    FileSettingsStorage, InMemorySettingsStorage, SettingsScope, SettingsStorage,
};
use super::types::*;

/// Manages global and project settings with deep merge.
pub struct SettingsManager {
    storage: Box<dyn SettingsStorage>,
    global_settings: Settings,
    project_settings: Settings,
    /// Merged view: project overrides global.
    pub settings: Settings,
    project_trusted: bool,
    global_error: Option<String>,
    project_error: Option<String>,
    errors: Arc<Mutex<Vec<(SettingsScope, String)>>>,
}

impl SettingsManager {
    // ── Construction ────────────────────────────────────────────

    /// Create from file storage (global ~/.xylitol/settings.json + project <cwd>/.xylitol/settings.json).
    pub fn from_files(
        cwd: &std::path::Path,
        agent_dir: &std::path::Path,
        project_trusted: bool,
    ) -> Self {
        let storage = FileSettingsStorage::new(cwd, agent_dir);
        Self::from_storage(Box::new(storage), project_trusted)
    }

    /// Create from an arbitrary storage backend (in-memory for tests).
    pub fn from_storage(storage: Box<dyn SettingsStorage>, project_trusted: bool) -> Self {
        let global = Self::load_from_storage(storage.as_ref(), SettingsScope::Global, true);
        let project =
            Self::load_from_storage(storage.as_ref(), SettingsScope::Project, project_trusted);
        let merged = Self::deep_merge(&global.0, &project.0);

        let mut errors = Vec::new();
        if let Some(ref e) = global.1 {
            errors.push((SettingsScope::Global, e.clone()));
        }
        if let Some(ref e) = project.1 {
            errors.push((SettingsScope::Project, e.clone()));
        }

        Self {
            storage,
            global_settings: global.0,
            project_settings: project.0,
            settings: merged,
            project_trusted,
            global_error: global.1,
            project_error: project.1,
            errors: Arc::new(Mutex::new(errors)),
        }
    }

    /// Create an in-memory manager from partial settings (testing).
    pub fn in_memory(settings: Settings) -> Self {
        let storage = Box::new(InMemorySettingsStorage::default());
        // Serialize once to populate
        let json = serde_json::to_string(&settings).unwrap();
        storage.with_lock(SettingsScope::Global, &mut |_cur| Some(json.clone()));
        Self::from_storage(storage, true)
    }

    fn load_from_storage(
        storage: &dyn SettingsStorage,
        scope: SettingsScope,
        trusted: bool,
    ) -> (Settings, Option<String>) {
        if matches!(scope, SettingsScope::Project) && !trusted {
            return (Settings::default(), None);
        }

        let mut result = None;
        let mut error = None;

        storage.with_lock(scope, &mut |current| {
            match current {
                Some(content) if !content.is_empty() => {
                    match serde_json::from_str::<Settings>(content) {
                        Ok(s) => result = Some(s),
                        Err(e) => {
                            error = Some(format!("parse settings {scope:?}: {e}"));
                        }
                    }
                }
                _ => {}
            }
            None // don't modify
        });

        (result.unwrap_or_default(), error)
    }

    // ── Deep merge ────────────────────────────────────────────

    /// Deep-merge two Settings structs: overrides win, None fields skip.
    /// For nested objects (compaction, retry, etc.), merge field-by-field.
    pub fn deep_merge(base: &Settings, overrides: &Settings) -> Settings {
        let mut result = base.clone();

        // Top-level non-object fields: override if Some
        if overrides.default_provider.is_some() {
            result.default_provider = overrides.default_provider.clone();
        }
        if overrides.default_model.is_some() {
            result.default_model = overrides.default_model.clone();
        }
        if overrides.default_thinking_level.is_some() {
            result.default_thinking_level = overrides.default_thinking_level.clone();
        }
        if overrides.theme.is_some() {
            result.theme = overrides.theme.clone();
        }
        if overrides.hide_thinking_block.is_some() {
            result.hide_thinking_block = overrides.hide_thinking_block;
        }
        if overrides.quiet_startup.is_some() {
            result.quiet_startup = overrides.quiet_startup;
        }
        if overrides.collapse_changelog.is_some() {
            result.collapse_changelog = overrides.collapse_changelog;
        }
        if overrides.enabled_models.is_some() {
            result.enabled_models = overrides.enabled_models.clone();
        }
        if overrides.extensions.is_some() {
            result.extensions = overrides.extensions.clone();
        }
        if overrides.skills.is_some() {
            result.skills = overrides.skills.clone();
        }
        if overrides.last_changelog_version.is_some() {
            result.last_changelog_version = overrides.last_changelog_version.clone();
        }

        // ── New fields (c89) ──
        if overrides.transport.is_some() {
            result.transport = overrides.transport.clone();
        }
        if overrides.steering_mode.is_some() {
            result.steering_mode = overrides.steering_mode.clone();
        }
        if overrides.follow_up_mode.is_some() {
            result.follow_up_mode = overrides.follow_up_mode.clone();
        }
        if overrides.shell_path.is_some() {
            result.shell_path = overrides.shell_path.clone();
        }
        if overrides.shell_command_prefix.is_some() {
            result.shell_command_prefix = overrides.shell_command_prefix.clone();
        }
        if overrides.npm_command.is_some() {
            result.npm_command = overrides.npm_command.clone();
        }
        if overrides.default_project_trust.is_some() {
            result.default_project_trust = overrides.default_project_trust.clone();
        }
        if overrides.enable_skill_commands.is_some() {
            result.enable_skill_commands = overrides.enable_skill_commands;
        }
        if overrides.prompts.is_some() {
            result.prompts = overrides.prompts.clone();
        }
        if overrides.themes.is_some() {
            result.themes = overrides.themes.clone();
        }
        if overrides.session_dir.is_some() {
            result.session_dir = overrides.session_dir.clone();
        }
        if overrides.http_proxy.is_some() {
            result.http_proxy = overrides.http_proxy.clone();
        }
        if overrides.http_idle_timeout_ms.is_some() {
            result.http_idle_timeout_ms = overrides.http_idle_timeout_ms;
        }
        if overrides.websocket_connect_timeout_ms.is_some() {
            result.websocket_connect_timeout_ms = overrides.websocket_connect_timeout_ms;
        }

        // Nested object merges
        merge_markdown(
            &mut result.markdown,
            base.markdown.as_ref(),
            overrides.markdown.as_ref(),
        );
        merge_warnings(
            &mut result.warnings,
            base.warnings.as_ref(),
            overrides.warnings.as_ref(),
        );
        merge_compaction(
            &mut result.compaction,
            base.compaction.as_ref(),
            overrides.compaction.as_ref(),
        );
        merge_branch_summary(
            &mut result.branch_summary,
            base.branch_summary.as_ref(),
            overrides.branch_summary.as_ref(),
        );
        merge_retry(
            &mut result.retry,
            base.retry.as_ref(),
            overrides.retry.as_ref(),
        );
        merge_terminal(
            &mut result.terminal,
            base.terminal.as_ref(),
            overrides.terminal.as_ref(),
        );
        merge_images(
            &mut result.images,
            base.images.as_ref(),
            overrides.images.as_ref(),
        );
        merge_thinking_budgets(
            &mut result.thinking_budgets,
            base.thinking_budgets.as_ref(),
            overrides.thinking_budgets.as_ref(),
        );
        merge_tools(
            &mut result.tools,
            base.tools.as_ref(),
            overrides.tools.as_ref(),
        );

        result
    }

    // ── Accessors (with defaults) ────────────────────────────

    pub fn get_default_provider(&self) -> Option<&str> {
        self.settings.default_provider.as_deref()
    }

    pub fn get_default_model(&self) -> Option<&str> {
        self.settings.default_model.as_deref()
    }

    pub fn get_compaction_enabled(&self) -> bool {
        self.settings
            .compaction
            .as_ref()
            .and_then(|c| c.enabled)
            .unwrap_or(true)
    }

    pub fn get_compaction_reserve_tokens(&self) -> u64 {
        self.settings
            .compaction
            .as_ref()
            .and_then(|c| c.reserve_tokens)
            .unwrap_or(16384)
    }

    pub fn get_compaction_keep_recent_tokens(&self) -> u64 {
        self.settings
            .compaction
            .as_ref()
            .and_then(|c| c.keep_recent_tokens)
            .unwrap_or(20000)
    }

    pub fn get_enabled_models(&self) -> Option<&[String]> {
        self.settings.enabled_models.as_deref()
    }

    pub fn get_retry_enabled(&self) -> bool {
        self.settings
            .retry
            .as_ref()
            .and_then(|r| r.enabled)
            .unwrap_or(true)
    }

    pub fn get_terminal_show_images(&self) -> bool {
        self.settings
            .terminal
            .as_ref()
            .and_then(|t| t.show_images)
            .unwrap_or(true)
    }

    pub fn get_block_images(&self) -> bool {
        self.settings
            .images
            .as_ref()
            .and_then(|i| i.block_images)
            .unwrap_or(false)
    }

    pub fn get_theme(&self) -> Option<&str> {
        self.settings.theme.as_deref()
    }

    pub fn get_thinking_budgets(&self) -> Option<&ThinkingBudgets> {
        self.settings.thinking_budgets.as_ref()
    }

    /// Get tools allow-list if defined.
    pub fn get_tools_allow(&self) -> Option<&[String]> {
        self.settings
            .tools
            .as_ref()
            .and_then(|t| t.allow.as_deref())
    }

    /// Get tools deny-list if defined.
    pub fn get_tools_deny(&self) -> Option<&[String]> {
        self.settings.tools.as_ref().and_then(|t| t.deny.as_deref())
    }

    // ── Mutators (global scope, persist immediately) ──────────

    fn save_global(&mut self) {
        let settings = self.global_settings.clone();
        let json = match serde_json::to_string_pretty(&settings) {
            Ok(j) => j,
            Err(e) => {
                tracing::warn!("serialize global settings: {e}");
                return;
            }
        };
        self.storage
            .with_lock(SettingsScope::Global, &mut |_cur| Some(json.clone()));
    }

    pub fn set_default_model(&mut self, model: &str) {
        self.global_settings.default_model = Some(model.to_string());
        self.settings = Self::deep_merge(&self.global_settings, &self.project_settings);
        self.save_global();
    }

    pub fn set_default_provider(&mut self, provider: &str) {
        self.global_settings.default_provider = Some(provider.to_string());
        self.settings = Self::deep_merge(&self.global_settings, &self.project_settings);
        self.save_global();
    }

    pub fn set_default_model_and_provider(&mut self, provider: &str, model: &str) {
        self.global_settings.default_provider = Some(provider.to_string());
        self.global_settings.default_model = Some(model.to_string());
        self.settings = Self::deep_merge(&self.global_settings, &self.project_settings);
        self.save_global();
    }

    pub fn set_compaction_enabled(&mut self, enabled: bool) {
        self.global_settings
            .compaction
            .get_or_insert_with(CompactionSettings::default)
            .enabled = Some(enabled);
        self.settings = Self::deep_merge(&self.global_settings, &self.project_settings);
        self.save_global();
    }

    pub fn set_theme(&mut self, theme: &str) {
        self.global_settings.theme = Some(theme.to_string());
        self.settings = Self::deep_merge(&self.global_settings, &self.project_settings);
        self.save_global();
    }

    // ── Reload ────────────────────────────────────────────────

    /// Reload from storage and rebuild merged view.
    /// Returns true if settings actually changed.
    pub fn reload(&mut self) -> bool {
        let new_global =
            Self::load_from_storage(self.storage.as_ref(), SettingsScope::Global, true);
        let new_project = Self::load_from_storage(
            self.storage.as_ref(),
            SettingsScope::Project,
            self.project_trusted,
        );

        let changed =
            self.global_settings != new_global.0 || self.project_settings != new_project.0;

        // Update errors
        {
            let mut errs = self.errors.lock().unwrap();
            errs.clear();
            if let Some(ref e) = new_global.1 {
                errs.push((SettingsScope::Global, e.clone()));
            }
            if let Some(ref e) = new_project.1 {
                errs.push((SettingsScope::Project, e.clone()));
            }
        }

        self.global_settings = new_global.0;
        self.project_settings = new_project.0;
        self.global_error = new_global.1;
        self.project_error = new_project.1;
        self.settings = Self::deep_merge(&self.global_settings, &self.project_settings);

        changed
    }

    /// Drain accumulated errors (e.g., parse failures from load/reload).
    pub fn drain_errors(&self) -> Vec<(SettingsScope, String)> {
        let mut errs = self.errors.lock().unwrap();
        std::mem::take(&mut *errs)
    }

    // ── Project trust ─────────────────────────────────────────

    pub fn is_project_trusted(&self) -> bool {
        self.project_trusted
    }

    pub fn set_project_trusted(&mut self, trusted: bool) {
        if self.project_trusted == trusted {
            return;
        }
        self.project_trusted = trusted;

        if !trusted {
            self.project_settings = Settings::default();
            self.project_error = None;
        } else {
            let (settings, error) =
                Self::load_from_storage(self.storage.as_ref(), SettingsScope::Project, true);
            self.project_settings = settings;
            self.project_error = error;
        }
        self.settings = Self::deep_merge(&self.global_settings, &self.project_settings);
    }

    // ── Raw access for callers that need structure ────────────

    pub fn get_settings(&self) -> &Settings {
        &self.settings
    }

    pub fn get_global_settings(&self) -> &Settings {
        &self.global_settings
    }

    pub fn get_project_settings(&self) -> &Settings {
        &self.project_settings
    }

    // ── New accessors (c89) ──

    pub fn get_transport(&self) -> Transport {
        self.settings.transport.clone().unwrap_or_default()
    }

    pub fn get_steering_mode(&self) -> SteeringMode {
        self.settings.steering_mode.clone().unwrap_or_default()
    }

    pub fn get_follow_up_mode(&self) -> SteeringMode {
        self.settings.follow_up_mode.clone().unwrap_or_default()
    }

    pub fn get_shell_path(&self) -> Option<&str> {
        self.settings.shell_path.as_deref()
    }

    pub fn get_shell_command_prefix(&self) -> Option<&str> {
        self.settings.shell_command_prefix.as_deref()
    }

    pub fn get_npm_command(&self) -> Option<&[String]> {
        self.settings.npm_command.as_deref()
    }

    pub fn get_default_project_trust(&self) -> DefaultProjectTrust {
        self.settings
            .default_project_trust
            .clone()
            .unwrap_or_default()
    }

    pub fn get_enable_skill_commands(&self) -> bool {
        self.settings.enable_skill_commands.unwrap_or(true)
    }

    pub fn get_prompts(&self) -> Option<&[String]> {
        self.settings.prompts.as_deref()
    }

    pub fn get_themes(&self) -> Option<&[String]> {
        self.settings.themes.as_deref()
    }

    pub fn get_session_dir(&self) -> Option<&str> {
        self.settings.session_dir.as_deref()
    }

    pub fn get_http_proxy(&self) -> Option<&str> {
        self.settings.http_proxy.as_deref()
    }

    pub fn get_http_idle_timeout_ms(&self) -> Option<u64> {
        self.settings.http_idle_timeout_ms
    }

    pub fn get_websocket_connect_timeout_ms(&self) -> Option<u64> {
        self.settings.websocket_connect_timeout_ms
    }
}

// ── Private: nested merge helpers ─────────────────────────────────

fn merge_compaction(
    target: &mut Option<CompactionSettings>,
    base: Option<&CompactionSettings>,
    overrides: Option<&CompactionSettings>,
) {
    let b = match base {
        Some(b) => b,
        None => {
            *target = overrides.cloned();
            return;
        }
    };
    let o = match overrides {
        Some(o) => o,
        None => {
            *target = Some(b.clone());
            return;
        }
    };
    *target = Some(CompactionSettings {
        enabled: o.enabled.or(b.enabled),
        reserve_tokens: o.reserve_tokens.or(b.reserve_tokens),
        keep_recent_tokens: o.keep_recent_tokens.or(b.keep_recent_tokens),
    });
}

fn merge_branch_summary(
    target: &mut Option<BranchSummarySettings>,
    base: Option<&BranchSummarySettings>,
    overrides: Option<&BranchSummarySettings>,
) {
    let b = match base {
        Some(b) => b,
        None => {
            *target = overrides.cloned();
            return;
        }
    };
    let o = match overrides {
        Some(o) => o,
        None => {
            *target = Some(b.clone());
            return;
        }
    };
    *target = Some(BranchSummarySettings {
        reserve_tokens: o.reserve_tokens.or(b.reserve_tokens),
        skip_prompt: o.skip_prompt.or(b.skip_prompt),
    });
}

fn merge_retry(
    target: &mut Option<RetrySettings>,
    base: Option<&RetrySettings>,
    overrides: Option<&RetrySettings>,
) {
    let b = match base {
        Some(b) => b,
        None => {
            *target = overrides.cloned();
            return;
        }
    };
    let o = match overrides {
        Some(o) => o,
        None => {
            *target = Some(b.clone());
            return;
        }
    };
    *target = Some(RetrySettings {
        enabled: o.enabled.or(b.enabled),
        max_retries: o.max_retries.or(b.max_retries),
        base_delay_ms: o.base_delay_ms.or(b.base_delay_ms),
        provider: merge_provider_retry(b.provider.as_ref(), o.provider.as_ref()),
    });
}

fn merge_provider_retry(
    base: Option<&ProviderRetrySettings>,
    overrides: Option<&ProviderRetrySettings>,
) -> Option<ProviderRetrySettings> {
    match (base, overrides) {
        (None, None) => None,
        (Some(b), None) => Some(b.clone()),
        (None, Some(o)) => Some(o.clone()),
        (Some(b), Some(o)) => Some(ProviderRetrySettings {
            timeout_ms: o.timeout_ms.or(b.timeout_ms),
            max_retries: o.max_retries.or(b.max_retries),
            max_retry_delay_ms: o.max_retry_delay_ms.or(b.max_retry_delay_ms),
        }),
    }
}

fn merge_terminal(
    target: &mut Option<TerminalSettings>,
    base: Option<&TerminalSettings>,
    overrides: Option<&TerminalSettings>,
) {
    let b = match base {
        Some(b) => b,
        None => {
            *target = overrides.cloned();
            return;
        }
    };
    let o = match overrides {
        Some(o) => o,
        None => {
            *target = Some(b.clone());
            return;
        }
    };
    *target = Some(TerminalSettings {
        show_images: o.show_images.or(b.show_images),
        image_width_cells: o.image_width_cells.or(b.image_width_cells),
        clear_on_shrink: o.clear_on_shrink.or(b.clear_on_shrink),
    });
}

fn merge_images(
    target: &mut Option<ImageSettings>,
    base: Option<&ImageSettings>,
    overrides: Option<&ImageSettings>,
) {
    let b = match base {
        Some(b) => b,
        None => {
            *target = overrides.cloned();
            return;
        }
    };
    let o = match overrides {
        Some(o) => o,
        None => {
            *target = Some(b.clone());
            return;
        }
    };
    *target = Some(ImageSettings {
        auto_resize: o.auto_resize.or(b.auto_resize),
        block_images: o.block_images.or(b.block_images),
    });
}

fn merge_thinking_budgets(
    target: &mut Option<ThinkingBudgets>,
    base: Option<&ThinkingBudgets>,
    overrides: Option<&ThinkingBudgets>,
) {
    let b = match base {
        Some(b) => b,
        None => {
            *target = overrides.cloned();
            return;
        }
    };
    let o = match overrides {
        Some(o) => o,
        None => {
            *target = Some(b.clone());
            return;
        }
    };
    *target = Some(ThinkingBudgets {
        minimal: o.minimal.or(b.minimal),
        low: o.low.or(b.low),
        medium: o.medium.or(b.medium),
        high: o.high.or(b.high),
    });
}

fn merge_markdown(
    target: &mut Option<MarkdownSettings>,
    base: Option<&MarkdownSettings>,
    overrides: Option<&MarkdownSettings>,
) {
    let b = match base {
        Some(b) => b,
        None => {
            *target = overrides.cloned();
            return;
        }
    };
    let o = match overrides {
        Some(o) => o,
        None => {
            *target = Some(b.clone());
            return;
        }
    };
    *target = Some(MarkdownSettings {
        code_block_indent: o
            .code_block_indent
            .clone()
            .or_else(|| b.code_block_indent.clone()),
    });
}

fn merge_warnings(
    target: &mut Option<WarningSettings>,
    base: Option<&WarningSettings>,
    overrides: Option<&WarningSettings>,
) {
    let b = match base {
        Some(b) => b,
        None => {
            *target = overrides.cloned();
            return;
        }
    };
    let o = match overrides {
        Some(o) => o,
        None => {
            *target = Some(b.clone());
            return;
        }
    };
    *target = Some(WarningSettings {
        anthropic_extra_usage: o.anthropic_extra_usage.or(b.anthropic_extra_usage),
    });
}

fn merge_tools(
    target: &mut Option<ToolsAllowDeny>,
    base: Option<&ToolsAllowDeny>,
    overrides: Option<&ToolsAllowDeny>,
) {
    let b = match base {
        Some(b) => b,
        None => {
            *target = overrides.cloned();
            return;
        }
    };
    let o = match overrides {
        Some(o) => o,
        None => {
            *target = Some(b.clone());
            return;
        }
    };
    *target = Some(ToolsAllowDeny {
        allow: o.allow.clone().or_else(|| b.allow.clone()),
        deny: o.deny.clone().or_else(|| b.deny.clone()),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deep_merge_override() {
        let base = Settings {
            compaction: Some(CompactionSettings {
                enabled: Some(true),
                reserve_tokens: Some(16384),
                keep_recent_tokens: Some(20000),
            }),
            ..Default::default()
        };
        let overrides = Settings {
            compaction: Some(CompactionSettings {
                enabled: Some(false),
                reserve_tokens: None,
                keep_recent_tokens: None,
            }),
            ..Default::default()
        };

        let merged = SettingsManager::deep_merge(&base, &overrides);
        assert_eq!(merged.compaction.as_ref().unwrap().enabled, Some(false));
        assert_eq!(
            merged.compaction.as_ref().unwrap().reserve_tokens,
            Some(16384)
        );
        assert_eq!(
            merged.compaction.as_ref().unwrap().keep_recent_tokens,
            Some(20000)
        );
    }

    #[test]
    fn test_deep_merge_no_project() {
        let base = Settings {
            default_model: Some("gpt-4o".into()),
            ..Default::default()
        };
        let project = Settings::default();
        let merged = SettingsManager::deep_merge(&base, &project);
        assert_eq!(merged.default_model.as_deref(), Some("gpt-4o"));
    }

    #[test]
    fn test_in_memory_roundtrip() {
        let mgr = SettingsManager::in_memory(Settings {
            default_model: Some("gpt-4o".into()),
            ..Default::default()
        });
        assert_eq!(mgr.get_default_model(), Some("gpt-4o"));
        assert!(mgr.get_compaction_enabled());
    }

    #[test]
    fn test_set_and_get() {
        let mut mgr = SettingsManager::in_memory(Default::default());
        mgr.set_default_model("claude-3");
        assert_eq!(mgr.get_default_model(), Some("claude-3"));
    }

    #[test]
    fn test_compaction_defaults() {
        let mgr = SettingsManager::in_memory(Default::default());
        assert!(mgr.get_compaction_enabled());
        assert_eq!(mgr.get_compaction_reserve_tokens(), 16384);
        assert_eq!(mgr.get_compaction_keep_recent_tokens(), 20000);
    }

    #[test]
    fn test_tools_allow_deny() {
        let mgr = SettingsManager::in_memory(Settings {
            tools: Some(ToolsAllowDeny {
                allow: Some(vec!["read".into(), "bash".into()]),
                deny: None,
            }),
            ..Default::default()
        });
        let allow = mgr.get_tools_allow().unwrap();
        assert!(allow.contains(&"read".to_string()));
        assert!(allow.contains(&"bash".to_string()));
    }

    #[test]
    fn test_tools_deny() {
        let mgr = SettingsManager::in_memory(Settings {
            tools: Some(ToolsAllowDeny {
                allow: None,
                deny: Some(vec!["bash".into()]),
            }),
            ..Default::default()
        });
        let deny = mgr.get_tools_deny().unwrap();
        assert_eq!(deny, &["bash"]);
    }

    #[test]
    fn test_reload_no_change() {
        let mut mgr = SettingsManager::in_memory(Settings {
            default_model: Some("gpt-4o".into()),
            ..Default::default()
        });
        let changed = mgr.reload();
        assert!(!changed);
    }

    #[test]
    fn test_project_trust_toggle() {
        let mut mgr = SettingsManager::in_memory(Settings {
            default_model: Some("gpt-4o".into()),
            ..Default::default()
        });
        assert!(mgr.is_project_trusted());
        mgr.set_project_trusted(false);
        assert!(!mgr.is_project_trusted());
        // Re-enabling should re-load
        mgr.set_project_trusted(true);
        assert!(mgr.is_project_trusted());
        assert_eq!(mgr.get_default_model(), Some("gpt-4o"));
    }

    #[test]
    fn test_project_trust_untrusted_clears_project_settings() {
        // t5 characterization: when project becomes untrusted, the project-scoped
        // settings MUST be dropped and the effective settings MUST reflect only
        // global settings (no project merge). This locks the behavior that 2.B.6
        // will wire to the resolved trust decision.
        use crate::infra::settings::storage::InMemorySettingsStorage;

        let storage = InMemorySettingsStorage::default();
        // Populate global and project scopes separately.
        let global_json = serde_json::to_string(&Settings {
            default_model: Some("global-model".into()),
            ..Default::default()
        })
        .unwrap();
        let project_json = serde_json::to_string(&Settings {
            default_model: Some("project-model".into()),
            ..Default::default()
        })
        .unwrap();
        storage.global.lock().unwrap().replace(global_json);
        storage.project.lock().unwrap().replace(project_json);

        let mut mgr = SettingsManager::from_storage(Box::new(storage), true);
        // Initially trusted: effective default_model comes from project merge.
        assert!(mgr.is_project_trusted());
        assert_eq!(mgr.get_default_model(), Some("project-model".into()));

        // Flip to untrusted: project settings cleared, global wins.
        mgr.set_project_trusted(false);
        assert!(!mgr.is_project_trusted());
        assert_eq!(
            mgr.get_default_model(),
            Some("global-model".into()),
            "untrusted project MUST NOT contribute to effective settings"
        );
        assert_eq!(mgr.get_project_settings(), &Settings::default());
    }
}
