//! Settings types mirroring pi's Settings interface.
//!
//! All fields use `Option` for partial overrides. The merged view
//! provides defaults via accessor methods on SettingsManager.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// User-controlled settings that can live in global (~/.xylitol/settings.json)
/// or project (<cwd>/.xylitol/settings.json).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_changelog_version: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_provider: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_model: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_thinking_level: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub compaction: Option<CompactionSettings>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_summary: Option<BranchSummarySettings>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<RetrySettings>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub hide_thinking_block: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub quiet_startup: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub collapse_changelog: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal: Option<TerminalSettings>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<ImageSettings>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled_models: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub skills: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_budgets: Option<ThinkingBudgets>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsAllowDeny>,

    // ── New fields (c89) ──
    /// Transport mode (auto / sse / direct).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport: Option<Transport>,

    /// Steering mode for follow-up messages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steering_mode: Option<SteeringMode>,

    /// Follow-up mode for queue processing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub follow_up_mode: Option<SteeringMode>,

    /// Custom shell path (e.g., for Cygwin users on Windows).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shell_path: Option<String>,

    /// Prefix prepended to every bash command.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shell_command_prefix: Option<String>,

    /// Command used for npm package operations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub npm_command: Option<Vec<String>>,

    /// Default project trust behaviour.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_project_trust: Option<DefaultProjectTrust>,

    /// Register skills as /skill:name commands.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_skill_commands: Option<bool>,

    /// Custom prompt template paths.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<Vec<String>>,

    /// Custom theme paths.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub themes: Option<Vec<String>>,

    /// Custom session storage directory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_dir: Option<String>,

    /// HTTP proxy URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_proxy: Option<String>,

    /// HTTP idle timeout in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_idle_timeout_ms: Option<u64>,

    /// WebSocket connect timeout in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub websocket_connect_timeout_ms: Option<u64>,

    /// Markdown rendering options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<MarkdownSettings>,

    /// Warning toggles.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<WarningSettings>,
}

/// Compaction behavior settings — relocated to `domain::compaction_config`.
pub use crate::domain::compaction_config::CompactionSettings;

/// Branch summary settings for forked conversations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
#[derive(Default)]
pub struct BranchSummarySettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reserve_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_prompt: Option<bool>,
}

/// Retry behavior for agent loop failures.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
#[derive(Default)]
pub struct RetrySettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_retries: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_delay_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<ProviderRetrySettings>,
}

/// Provider-level retry settings (HTTP 429/5xx).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
#[derive(Default)]
pub struct ProviderRetrySettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_retries: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_retry_delay_ms: Option<u64>,
}

/// Terminal display settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
#[derive(Default)]
pub struct TerminalSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_images: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_width_cells: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clear_on_shrink: Option<bool>,
}

/// Image processing settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
#[derive(Default)]
pub struct ImageSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_resize: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_images: Option<bool>,
}

/// Thinking budget tokens per level.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
#[derive(Default)]
pub struct ThinkingBudgets {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimal: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub low: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub medium: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub high: Option<u64>,
}

/// Transport mode.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Transport {
    #[default]
    Auto,
    Sse,
    Direct,
}

/// Steering / follow-up mode.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum SteeringMode {
    #[default]
    All,
    #[serde(rename = "one-at-a-time")]
    OneAtATime,
}

/// Default project trust policy.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum DefaultProjectTrust {
    #[default]
    Ask,
    Always,
    Never,
}

/// Markdown rendering options.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
#[derive(Default)]
pub struct MarkdownSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_block_indent: Option<String>,
}

/// Warning toggles.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
#[derive(Default)]
pub struct WarningSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anthropic_extra_usage: Option<bool>,
}

/// Tool allow/deny lists for scope control.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
#[derive(Default)]
pub struct ToolsAllowDeny {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deny: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_settings_camelcase() {
        let settings = Settings {
            default_provider: Some("anthropic".into()),
            compaction: Some(CompactionSettings {
                enabled: Some(false),
                reserve_tokens: None,
                keep_recent_tokens: Some(32768),
            }),
            ..Default::default()
        };
        let json = serde_json::to_string(&settings).unwrap();
        // pi-style: camelCase keys
        assert!(json.contains("defaultProvider"));
        assert!(json.contains("keepRecentTokens"));
        assert!(!json.contains("default_provider"));
    }
}
