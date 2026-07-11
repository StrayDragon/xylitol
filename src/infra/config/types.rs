//! AppConfig type definitions with serde + schemars derives.

use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Top-level config
// ---------------------------------------------------------------------------

/// The root configuration object for xylitol.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AppConfig {
    #[serde(rename = "models")]
    #[schemars(rename = "models")]
    pub model: ModelsConfig,
    pub agents: AgentsConfig,
    pub execution: ExecutionConfig,
    pub patch_apply: PatchApplyConfig,

    // ── always compiled ──────────────────────────────────────────────
    pub hooks: HooksConfig,
    pub security: SecurityConfig,
    pub repeat_detection: RepeatDetectionConfig,
    pub tools: ToolsConfig,

    // ── always compiled ────────────────────────────────────────────────
    pub session: Option<SessionConfig>,
    /// YAML loading-phase compaction config. Mapped to runtime
    /// `CompactionSettings` (in `agent::compaction::settings`)
    /// via `From<XyCompactionSettingsConfig>`.
    /// Schema twin: [`CompactionSettingsSchema`] (domain type is serde-only).
    #[schemars(with = "Option<CompactionSettingsSchema>")]
    pub compaction: Option<crate::domain::compaction_config::XyCompactionSettingsConfig>,

    pub skills: Option<Vec<SkillConfig>>,
    pub mcp_servers: Option<Vec<McpServerConfig>>,

    pub review: Option<ReviewConfig>,
}

// ---------------------------------------------------------------------------
// Model
// ---------------------------------------------------------------------------

/// Top-level model configuration section.
/// Split from agent-level [`XyModelConfig`](crate::domain::model::XyModelConfig) —
/// this is YAML-facing; [`ModelEntry`] aliases resolve into runtime config.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct ModelsConfig {
    /// Default model ID (or alias key) to use when no model is specified.
    pub default_model: Option<String>,
    /// Named model entries keyed by alias.
    pub models: HashMap<String, ModelEntry>,
}

/// A single model alias entry.
///
/// References [`XyModelKind`](crate::domain::model::XyModelKind) for the provider;
/// the kind's serde representation is the YAML wire format. Schema uses a string
/// twin so domain stays free of schemars (c510).
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct ModelEntry {
    #[schemars(with = "String")]
    pub provider: crate::domain::model::XyModelKind,
    pub model: String,
    /// Optional custom base URL for OpenAI-compatible or Anthropic-compatible APIs.
    #[serde(default)]
    pub base_url: Option<String>,
    /// Optional adapter API type, e.g. `openai-responses` or `openai-completions`.
    #[serde(default)]
    pub api: Option<String>,
    /// Optional fallback model ID (must be another key in `models`).
    #[serde(default)]
    pub fallback: Option<String>,
    /// Whether this model supports thinking/reasoning. Default: true for all.
    #[serde(default = "default_thinking")]
    pub thinking: bool,
    /// Context window size in tokens. Default: 0 (auto-detect from provider).
    #[serde(default)]
    pub context_window: u64,
}

fn default_thinking() -> bool {
    true
}

// ---------------------------------------------------------------------------
// Execution
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct ExecutionConfig {
    /// Override model for execution steps.
    #[serde(default)]
    pub model: Option<String>,
    /// System prompt for the execution agent.
    #[serde(default)]
    pub system_prompt: Option<String>,
    /// Maximum retries on failure.
    #[serde(default = "default_max_retries")]
    pub max_retries: u8,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            model: None,
            system_prompt: None,
            max_retries: default_max_retries(),
        }
    }
}

fn default_max_retries() -> u8 {
    3
}

// ---------------------------------------------------------------------------
// Agent Profiles
// ---------------------------------------------------------------------------

/// Agent profile — binds model, prompt, tools, iterations.
///
/// Flat struct to support YAML anchor/alias merge keys:
/// ```yaml
/// agents:
///   profiles:
///     default: &default-agent
///       model: gpt-4o
///       max_iterations: 50
///     planning:
///       <<: *default-agent
///       model: claude-opus
///       system_prompt: "You are a planning agent."
/// ```
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct AgentProfile {
    /// Model alias referencing a key in `model.models`, or a raw model ID.
    /// When `None`, falls back to config-level defaults.
    #[serde(default)]
    pub model: Option<String>,
    /// System prompt / instruction for this agent.
    #[serde(default)]
    pub system_prompt: Option<String>,
    /// Tool names this agent is allowed to use. `None` = all tools.
    #[serde(default)]
    pub allowed_tools: Option<Vec<String>>,
    /// Maximum agent loop iterations.
    #[serde(default = "default_max_iterations")]
    pub max_iterations: u32,
}

/// Agent profiles container.
///
/// When entirely absent from config, falls back to single-model behavior
/// using config-level model resolution.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AgentsConfig {
    /// The profile name used when no profile is explicitly specified.
    #[serde(default = "default_profile_name")]
    pub default_profile: String,
    /// Named agent profiles.
    #[serde(default)]
    pub profiles: HashMap<String, AgentProfile>,
}

fn default_profile_name() -> String {
    "default".into()
}

fn default_max_iterations() -> u32 {
    50
}

impl AppConfig {
    /// Resolve a model alias to a runtime [`XyModelConfig`](crate::domain::model::XyModelConfig).
    pub fn resolve_model(
        &self,
        model_id: &str,
    ) -> Result<crate::domain::model::XyModelConfig, String> {
        use crate::domain::model::{XyModelConfig, XyModelKind};

        let (kind, model_name, base_url, api) = if let Some(entry) = self.model.models.get(model_id)
        {
            (
                entry.provider,
                entry.model.clone(),
                entry.base_url.clone(),
                entry.api.clone(),
            )
        } else {
            (XyModelKind::OpenAi, model_id.to_string(), None, None)
        };

        let api_key = match kind {
            XyModelKind::OpenAi => std::env::var("OPENAI_API_KEY")
                .or_else(|_| std::env::var("OPENAI_KEY"))
                .map_err(|_| "OPENAI_API_KEY environment variable is not set".to_string())?,
            XyModelKind::Anthropic => std::env::var("ANTHROPIC_API_KEY")
                .or_else(|_| std::env::var("ANTHROPIC_KEY"))
                .map_err(|_| "ANTHROPIC_API_KEY environment variable is not set".to_string())?,
            XyModelKind::Fake => String::new(),
        };

        Ok(XyModelConfig {
            kind,
            api_key,
            model: model_name,
            base_url,
            api,
        })
    }

    /// Resolve a model alias to [`XyModelMeta`](crate::domain::types::XyModelMeta) for the registry.
    ///
    /// Composes [`resolve_model`](Self::resolve_model) with per‑model metadata (thinking support,
    /// context window size) from [`ModelEntry`] or sensible defaults.
    pub fn resolve_model_meta(
        &self,
        model_id: &str,
    ) -> Result<crate::domain::types::XyModelMeta, String> {
        use crate::domain::model::default_context_window_for;
        use crate::domain::types::XyModelMeta;

        let model_config = self.resolve_model(model_id)?;
        let entry = self.model.models.get(model_id);

        let thinking = entry.map(|e| e.thinking).unwrap_or(true);
        let context_window = entry
            .and_then(|e| (e.context_window > 0).then_some(e.context_window))
            .unwrap_or_else(|| default_context_window_for(model_config.kind));

        Ok(XyModelMeta {
            id: model_id.to_string(),
            config: model_config,
            display_name: model_id.to_string(),
            thinking,
            context_window,
            api: String::new(),
            provider: String::new(),
            cost_input: 0.0,
            cost_output: 0.0,
            cost_cache_read: 0.0,
            cost_cache_write: 0.0,
            max_tokens: 0,
            thinking_levels: Vec::new(),
        })
    }

    /// Resolve a named agent profile to a [`ResolvedProfile`].
    pub fn resolve_profile(
        &self,
        name: &str,
    ) -> Result<crate::domain::model::ResolvedProfile, String> {
        let profile = self.agents.profiles.get(name);

        let (model_ref, system_prompt, allowed_tools, max_iterations) = match profile {
            Some(p) => (
                p.model.as_deref(),
                p.system_prompt.as_ref().cloned(),
                p.allowed_tools.as_ref().cloned(),
                p.max_iterations,
            ),
            None => (None, self.execution.system_prompt.clone(), None, 50),
        };

        let model_id = model_ref
            .or(self.execution.model.as_deref())
            .or(self.model.default_model.as_deref())
            .ok_or_else(|| {
                "no model configured: set `--model`, `execution.model`, `model.default_model`, \
                 or `agents.profiles.<name>.model`"
                    .to_string()
            })?;
        let model_config = self.resolve_model(model_id)?;

        Ok(crate::domain::model::ResolvedProfile {
            model_config,
            system_prompt,
            allowed_tools,
            max_iterations,
            name: name.into(),
        })
    }

    /// Resolve the default agent profile.
    pub fn resolve_default_profile(&self) -> Result<crate::domain::model::ResolvedProfile, String> {
        let name = if self.agents.default_profile.is_empty() {
            "default"
        } else {
            &self.agents.default_profile
        };
        self.resolve_profile(name)
    }
}

// ---------------------------------------------------------------------------
// Patch apply
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct PatchApplyConfig {
    /// Whether to automatically apply patches without prompting.
    #[serde(default)]
    pub auto_apply: bool,
}

// ---------------------------------------------------------------------------
// Hooks
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct HooksConfig {
    pub global: Vec<HookEntry>,
    pub project: Vec<HookEntry>,
    pub user: Vec<HookEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct HookEntry {
    /// Shell command to execute (e.g., "python3 /path/to/hook.py").
    pub command: String,
    /// Event patterns to match (e.g., "pre.tool_call", "post.step_complete").
    /// Supports exact and prefix matching.
    #[serde(default)]
    pub events: Vec<String>,
    /// Timeout in seconds per hook execution. Default: 5.
    #[serde(default = "default_hook_timeout")]
    pub timeout_secs: u64,
    /// Optional phase filter: "pre", "post", or "" for both. When set, only
    /// matches events of that phase.
    #[serde(default)]
    pub phase: String,
    /// Whether this hook requires explicit user approval before execution.
    #[serde(default)]
    pub requires_approval: bool,
    /// Extra environment variables for the hook process.
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
}

impl Default for HookEntry {
    fn default() -> Self {
        Self {
            command: String::new(),
            events: Vec::new(),
            timeout_secs: default_hook_timeout(),
            phase: String::new(),
            requires_approval: false,
            env: std::collections::HashMap::new(),
        }
    }
}

fn default_hook_timeout() -> u64 {
    5
}

// ---------------------------------------------------------------------------
// Security
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct SecurityConfig {
    /// Master toggle — enabled by default for safety.
    #[serde(default = "default_security_enabled")]
    pub enabled: bool,
    pub permission: Option<PermissionConfig>,
}

fn default_security_enabled() -> bool {
    true
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct PermissionConfig {
    /// Master toggle.
    #[serde(default)]
    pub enabled: bool,
    /// Backend selection. "glob" (app-level) is always available;
    /// "landlock" requires Linux >=5.13; "macos" requires macOS.
    /// Default: "glob".
    #[serde(default = "default_permission_backend")]
    pub backend: PermissionBackend,
    /// Filesystem permission rules.
    #[serde(default)]
    pub filesystem: PermissionFilesystemConfig,
    /// Network permission rules (applied to bash URLs).
    #[serde(default)]
    pub network: PermissionNetworkConfig,
    /// Process execution rules.
    #[serde(default)]
    pub process: PermissionProcessConfig,
}

fn default_permission_backend() -> PermissionBackend {
    PermissionBackend::Glob
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum PermissionBackend {
    /// Pure application-level glob pattern matching (default).
    #[default]
    Glob,
    /// Linux Landlock LSM (requires kernel >=5.13).
    Landlock,
    /// macOS sandbox-init / Seatbelt.
    MacOs,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct PermissionFilesystemConfig {
    /// Paths allowed for read. Empty = allow all.
    #[serde(default)]
    pub read_allowed: Vec<String>,
    /// Paths denied for write. Write always denied when read_allowed is set
    /// and path is not in read_allowed (default-deny).
    #[serde(default)]
    pub write_allowed: Vec<String>,
    /// Paths explicitly denied for write (takes precedence over allowed).
    #[serde(default)]
    pub write_denied: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct PermissionNetworkConfig {
    /// Domains allowed for network requests. Empty = allow all.
    #[serde(default)]
    pub allowed_domains: Vec<String>,
    /// Domains explicitly denied.
    #[serde(default)]
    pub denied_domains: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct PermissionProcessConfig {
    /// Paths where subprocess execution is allowed. Empty = allow all.
    #[serde(default)]
    pub allowed_paths: Vec<String>,
}

// ---------------------------------------------------------------------------
// Repeat detection
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct RepeatDetectionConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_min_n")]
    pub min_n: u8,
    #[serde(default = "default_max_n")]
    pub max_n: u8,
    #[serde(default = "default_window_size")]
    pub window_size: u16,
    #[serde(default = "default_hit_threshold")]
    pub consecutive_hit_threshold: u8,
    #[serde(default = "default_window_repeat_ratio")]
    pub window_repeat_ratio: f64,
    /// Stop monitoring after this many tokens. 0 = no limit.
    #[serde(default)]
    pub early_stop_tokens: u16,
    #[serde(default)]
    pub recovery: RecoveryConfig,
}

impl Default for RepeatDetectionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            min_n: default_min_n(),
            max_n: default_max_n(),
            window_size: default_window_size(),
            consecutive_hit_threshold: default_hit_threshold(),
            window_repeat_ratio: default_window_repeat_ratio(),
            early_stop_tokens: 0,
            recovery: RecoveryConfig::default(),
        }
    }
}

fn default_min_n() -> u8 {
    3
}
fn default_max_n() -> u8 {
    10
}
fn default_window_size() -> u16 {
    100
}
fn default_hit_threshold() -> u8 {
    3
}
fn default_window_repeat_ratio() -> f64 {
    0.8
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct RecoveryConfig {
    #[serde(default = "default_recovery_strategy")]
    pub strategy: String,
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u8,
    #[serde(default)]
    pub actions: Vec<RecoveryAction>,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            strategy: default_recovery_strategy(),
            max_attempts: default_max_attempts(),
            actions: default_recovery_actions(),
        }
    }
}

/// A single recovery action in the sequential chain.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RecoveryAction {
    /// Prepend an anti-repetition warning to the prompt and retry.
    AlterPrompt {
        #[serde(default = "default_alter_prompt_prepend")]
        prepend: String,
    },
    /// Switch to a different model provider and retry.
    SwitchModel {
        /// Model ID to switch to. None = switch to the other provider.
        #[serde(default)]
        model_id: Option<String>,
    },
    /// Increase repetition/frequency/presence penalties and retry.
    AdjustParams {
        #[serde(default)]
        repetition_penalty: f64,
        #[serde(default)]
        frequency_penalty: f64,
        #[serde(default)]
        presence_penalty: f64,
    },
    /// Fall back to the planner for re-planning the task.
    DelegateToPlanner,
}

fn default_recovery_strategy() -> String {
    "sequential".into()
}
fn default_max_attempts() -> u8 {
    3
}
fn default_alter_prompt_prepend() -> String {
    "WARNING: Avoid repetition.".into()
}

fn default_recovery_actions() -> Vec<RecoveryAction> {
    vec![
        RecoveryAction::AlterPrompt {
            prepend: default_alter_prompt_prepend(),
        },
        RecoveryAction::AdjustParams {
            repetition_penalty: 1.4,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
        },
        RecoveryAction::DelegateToPlanner,
    ]
}

// ---------------------------------------------------------------------------
// Tools
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct ToolsConfig {
    #[serde(default)]
    pub allowlist: Vec<String>,
    #[serde(default)]
    pub blocklist: Vec<String>,

    #[serde(default = "default_max_results")]
    pub max_results: u32,
    #[serde(default = "default_max_file_size")]
    pub max_file_size: u64,
    #[serde(default = "default_max_dir_entries")]
    pub max_dir_entries: u32,
}

fn default_max_results() -> u32 {
    1000
}

fn default_max_file_size() -> u64 {
    10 * 1024 * 1024
}

fn default_max_dir_entries() -> u32 {
    1000
}

// ---------------------------------------------------------------------------
// Session & Compaction
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct SessionConfig {
    #[serde(default)]
    pub auto_snapshot: bool,
    #[serde(default = "default_max_snapshots")]
    pub max_snapshots: u16,
    pub storage: SessionStorageConfig,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            auto_snapshot: false,
            max_snapshots: default_max_snapshots(),
            storage: SessionStorageConfig::default(),
        }
    }
}

fn default_max_snapshots() -> u16 {
    50
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct SessionStorageConfig {
    #[serde(default = "default_storage_backend")]
    pub backend: String,
    #[serde(default)]
    pub path: Option<String>,
}

fn default_storage_backend() -> String {
    "file".into()
}

// Compaction: domain type is serde-only; schema twin lives here for AppConfig.
pub use crate::domain::compaction_config::XyCompactionSettingsConfig;

/// JSON Schema twin of [`XyCompactionSettingsConfig`] (infra-only; c510).
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct CompactionSettingsSchema {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reserve_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_recent_tokens: Option<u64>,
}

// ---------------------------------------------------------------------------
// Skills & MCP
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct SkillConfig {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    /// System prompt fragment injected when this skill is activated.
    #[serde(default)]
    pub system_prompt_addon: Option<String>,
    /// Tool names this skill is allowed to use. `None` or empty = all tools.
    #[serde(default)]
    pub allowed_tools: Option<Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct McpServerConfig {
    pub name: String,
    /// Transport kind: "stdio" or "sse".
    #[serde(default = "default_mcp_transport")]
    pub transport: McpTransportKind,
    /// Command to spawn (stdio transport only).
    #[serde(default)]
    pub command: Option<String>,
    /// Arguments for the command (stdio transport only).
    #[serde(default)]
    pub args: Option<Vec<String>>,
    /// SSE endpoint URL (sse transport only).
    #[serde(default)]
    pub url: Option<String>,
    /// Extra environment variables for the child process (stdio transport only).
    #[serde(default)]
    pub env: Option<HashMap<String, String>>,
}

impl Default for McpServerConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            transport: default_mcp_transport(),
            command: None,
            args: None,
            url: None,
            env: None,
        }
    }
}

fn default_mcp_transport() -> McpTransportKind {
    McpTransportKind::Stdio
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum McpTransportKind {
    Stdio,
    Sse,
}

// ---------------------------------------------------------------------------
// Review
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct ReviewConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_review_mode")]
    pub mode: String,
    #[serde(default = "default_review_backend")]
    pub backend: String,
}

impl Default for ReviewConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: default_review_mode(),
            backend: default_review_backend(),
        }
    }
}

fn default_review_mode() -> String {
    "diff".into()
}

fn default_review_backend() -> String {
    "cli".into()
}
