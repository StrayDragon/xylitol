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
pub(crate) struct AppConfig {
    pub model: ModelConfig,
    pub agents: AgentsConfig,
    pub execution: ExecutionConfig,
    pub patch_apply: PatchApplyConfig,

    // ── always compiled ──────────────────────────────────────────────
    pub hooks: HooksConfig,
    pub security: SecurityConfig,
    pub repeat_detection: RepeatDetectionConfig,
    pub tools: ToolsConfig,

    // ── feature-gated ────────────────────────────────────────────────
    #[cfg(feature = "agent-planning")]
    pub planning: Option<PlanningConfig>,
    #[cfg(feature = "agent-planning")]
    pub validation: Option<ValidationConfig>,

    #[cfg(feature = "infra-session")]
    pub session: Option<SessionConfig>,
    #[cfg(feature = "infra-session")]
    pub compaction: Option<CompactionConfig>,

    #[cfg(feature = "infra-skills")]
    pub skills: Option<Vec<SkillConfig>>,
    #[cfg(feature = "infra-skills")]
    pub mcp_servers: Option<Vec<McpServerConfig>>,

    #[cfg(feature = "ui-review")]
    pub review: Option<ReviewConfig>,

    #[cfg(feature = "infra-acp")]
    pub acp: Option<AcpConfig>,
}

// ---------------------------------------------------------------------------
// Model
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub(crate) struct ModelConfig {
    /// Default model ID to use when no model is specified.
    #[serde(default = "default_model_id")]
    pub default_model: String,
    /// Named model entries keyed by alias.
    #[serde(default)]
    pub models: HashMap<String, ModelEntry>,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            default_model: default_model_id(),
            models: HashMap::new(),
        }
    }
}

fn default_model_id() -> String {
    "gpt-4o".into()
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub(crate) struct ModelEntry {
    pub provider: ProviderKind,
    pub model: String,
    /// Optional fallback model ID (must be another key in `models`).
    #[serde(default)]
    pub fallback: Option<String>,
}

/// Supported LLM providers (MVP: only OpenAI-compatible and Anthropic).
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProviderKind {
    OpenAI,
    Anthropic,
}

// ---------------------------------------------------------------------------
// Execution
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub(crate) struct ExecutionConfig {
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
pub(crate) struct AgentProfile {
    /// Model alias referencing a key in `model.models`, or a raw model ID.
    /// When `None`, falls back to `model.default_model`.
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
/// using `model.default_model` and `execution.*` fields.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub(crate) struct AgentsConfig {
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
    /// Resolve a model alias to an agent-level [`ModelConfig`].
    pub(crate) fn resolve_model(
        &self,
        model_id: &str,
    ) -> Result<crate::agent::model::ModelConfig, String> {
        use crate::agent::model::{ModelConfig, ModelKind};

        let (kind, model_name) = if let Some(entry) = self.model.models.get(model_id) {
            let kind = match entry.provider {
                ProviderKind::OpenAI => ModelKind::OpenAi,
                ProviderKind::Anthropic => ModelKind::Anthropic,
            };
            (kind, entry.model.clone())
        } else {
            (ModelKind::OpenAi, model_id.to_string())
        };

        let api_key = match kind {
            ModelKind::OpenAi => std::env::var("OPENAI_API_KEY")
                .or_else(|_| std::env::var("OPENAI_KEY"))
                .map_err(|_| "OPENAI_API_KEY environment variable is not set".to_string())?,
            ModelKind::Anthropic => std::env::var("ANTHROPIC_API_KEY")
                .or_else(|_| std::env::var("ANTHROPIC_KEY"))
                .map_err(|_| "ANTHROPIC_API_KEY environment variable is not set".to_string())?,
            #[cfg(feature = "dev-fake-provider")]
            ModelKind::Fake => String::new(),
        };

        Ok(ModelConfig {
            kind,
            api_key,
            model: model_name,
            base_url: None,
        })
    }

    /// Resolve a named agent profile to a [`ResolvedProfile`].
    ///
    /// Falls back to `model.default_model` + `execution.*` when no profiles
    /// are configured (backward compatible).
    pub(crate) fn resolve_profile(
        &self,
        name: &str,
    ) -> Result<crate::agent::profile::ResolvedProfile, String> {
        let profile = self.agents.profiles.get(name);

        let (model_ref, system_prompt, allowed_tools, max_iterations) = match profile {
            Some(p) => (
                p.model.as_deref(),
                p.system_prompt.as_ref().cloned(),
                p.allowed_tools.as_ref().cloned(),
                p.max_iterations,
            ),
            None => {
                // Backward-compat: synthesize from execution config + default model.
                (
                    self.execution
                        .model
                        .as_deref()
                        .or(Some(&self.model.default_model)),
                    self.execution.system_prompt.clone(),
                    None,
                    50,
                )
            }
        };

        let model_id = model_ref.unwrap_or(&self.model.default_model);
        let model_config = self.resolve_model(model_id)?;

        Ok(crate::agent::profile::ResolvedProfile {
            model_config,
            system_prompt,
            allowed_tools,
            max_iterations,
            name: name.into(),
        })
    }

    /// Resolve the default agent profile.
    pub(crate) fn resolve_default_profile(
        &self,
    ) -> Result<crate::agent::profile::ResolvedProfile, String> {
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
pub(crate) struct PatchApplyConfig {
    /// Whether to automatically apply patches without prompting.
    #[serde(default)]
    pub auto_apply: bool,
}

// ---------------------------------------------------------------------------
// Hooks
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub(crate) struct HooksConfig {
    pub global: Vec<HookEntry>,
    pub project: Vec<HookEntry>,
    pub user: Vec<HookEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub(crate) struct HookEntry {
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
pub(crate) struct SecurityConfig {
    /// Master toggle — opt-in (default false).
    #[serde(default)]
    pub enabled: bool,
    /// Tools explicitly allowed (empty = allow all not in blocklist).
    #[serde(default)]
    pub tool_allowlist: Vec<String>,
    pub bash: BashSecurityConfig,
    pub filesystem: FilesystemSecurityConfig,
    pub network: NetworkSecurityConfig,
    pub resource_limits: ResourceLimits,
    /// Only present when `infra-sandbox` feature is enabled.
    #[cfg(feature = "infra-sandbox")]
    pub sandbox: Option<SandboxConfig>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub(crate) struct BashSecurityConfig {
    #[serde(default)]
    pub allowed_paths: Vec<String>,
    #[serde(default)]
    pub forbidden_patterns: Vec<String>,
    #[serde(default = "default_bash_timeout")]
    pub timeout_secs: u64,
}

impl Default for BashSecurityConfig {
    fn default() -> Self {
        Self {
            allowed_paths: Vec::new(),
            forbidden_patterns: Vec::new(),
            timeout_secs: 120,
        }
    }
}

fn default_bash_timeout() -> u64 {
    120
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub(crate) struct FilesystemSecurityConfig {
    #[serde(default)]
    pub allowed_patterns: Vec<String>,
    #[serde(default)]
    pub forbidden_patterns: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub(crate) struct NetworkSecurityConfig {
    #[serde(default)]
    pub allowed_domains: Vec<String>,
    #[serde(default)]
    pub blocked_domains: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub(crate) struct ResourceLimits {
    #[serde(default = "default_max_subprocesses")]
    pub max_subprocesses: u16,
    #[serde(default = "default_max_memory")]
    pub max_memory_mb: u64,
    #[serde(default = "default_max_cpu")]
    pub max_cpu_percent: u8,
    #[serde(default = "default_max_disk")]
    pub max_disk_mb: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_subprocesses: default_max_subprocesses(),
            max_memory_mb: default_max_memory(),
            max_cpu_percent: default_max_cpu(),
            max_disk_mb: default_max_disk(),
        }
    }
}

fn default_max_subprocesses() -> u16 {
    16
}
fn default_max_memory() -> u64 {
    4096
}
fn default_max_cpu() -> u8 {
    80
}
fn default_max_disk() -> u64 {
    1024
}

#[cfg(feature = "infra-sandbox")]
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub(crate) struct SandboxConfig {
    #[serde(default)]
    pub enabled: bool,
}

// ---------------------------------------------------------------------------
// Repeat detection
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub(crate) struct RepeatDetectionConfig {
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
pub(crate) struct RecoveryConfig {
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
pub(crate) enum RecoveryAction {
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
pub(crate) struct ToolsConfig {
    #[serde(default)]
    pub allowlist: Vec<String>,
    #[serde(default)]
    pub blocklist: Vec<String>,
}

// ---------------------------------------------------------------------------
// Feature-gated: agent-planning
// ---------------------------------------------------------------------------

#[cfg(feature = "agent-planning")]
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub(crate) struct PlanningConfig {
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default = "default_max_steps")]
    pub max_steps: u16,
    #[serde(default = "default_reasoning_depth")]
    pub reasoning_depth: String,
}

#[cfg(feature = "agent-planning")]
impl Default for PlanningConfig {
    fn default() -> Self {
        Self {
            model: None,
            system_prompt: None,
            max_steps: default_max_steps(),
            reasoning_depth: default_reasoning_depth(),
        }
    }
}

#[cfg(feature = "agent-planning")]
fn default_max_steps() -> u16 {
    10
}

#[cfg(feature = "agent-planning")]
fn default_reasoning_depth() -> String {
    "medium".into()
}

#[cfg(feature = "agent-planning")]
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub(crate) struct ValidationConfig {
    #[serde(default)]
    pub enabled: bool,
}

// ---------------------------------------------------------------------------
// Feature-gated: infra-session
// ---------------------------------------------------------------------------

#[cfg(feature = "infra-session")]
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub(crate) struct SessionConfig {
    #[serde(default)]
    pub auto_snapshot: bool,
    #[serde(default = "default_max_snapshots")]
    pub max_snapshots: u16,
    pub storage: SessionStorageConfig,
}

#[cfg(feature = "infra-session")]
impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            auto_snapshot: false,
            max_snapshots: default_max_snapshots(),
            storage: SessionStorageConfig::default(),
        }
    }
}

#[cfg(feature = "infra-session")]
fn default_max_snapshots() -> u16 {
    50
}

#[cfg(feature = "infra-session")]
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub(crate) struct SessionStorageConfig {
    #[serde(default = "default_storage_backend")]
    pub backend: String,
    #[serde(default)]
    pub path: Option<String>,
}

#[cfg(feature = "infra-session")]
fn default_storage_backend() -> String {
    "file".into()
}

#[cfg(feature = "infra-session")]
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub(crate) struct CompactionConfig {
    #[serde(default)]
    pub enabled: bool,
}

// ---------------------------------------------------------------------------
// Feature-gated: infra-skills
// ---------------------------------------------------------------------------

#[cfg(feature = "infra-skills")]
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub(crate) struct SkillConfig {
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

#[cfg(feature = "infra-skills")]
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub(crate) struct McpServerConfig {
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

#[cfg(feature = "infra-skills")]
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

#[cfg(feature = "infra-skills")]
fn default_mcp_transport() -> McpTransportKind {
    McpTransportKind::Stdio
}

#[cfg(feature = "infra-skills")]
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum McpTransportKind {
    Stdio,
    Sse,
}

// ---------------------------------------------------------------------------
// Feature-gated: ui-review
// ---------------------------------------------------------------------------

#[cfg(feature = "ui-review")]
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub(crate) struct ReviewConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_review_mode")]
    pub mode: String,
    #[serde(default = "default_review_backend")]
    pub backend: String,
}

#[cfg(feature = "ui-review")]
impl Default for ReviewConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: default_review_mode(),
            backend: default_review_backend(),
        }
    }
}

#[cfg(feature = "ui-review")]
fn default_review_mode() -> String {
    "diff".into()
}

#[cfg(feature = "ui-review")]
fn default_review_backend() -> String {
    "cli".into()
}

// ---------------------------------------------------------------------------
// Feature-gated: infra-acp
// ---------------------------------------------------------------------------

#[cfg(feature = "infra-acp")]
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub(crate) struct AcpConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_acp_port")]
    pub port: u16,
}

#[cfg(feature = "infra-acp")]
impl Default for AcpConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: default_acp_port(),
        }
    }
}

#[cfg(feature = "infra-acp")]
fn default_acp_port() -> u16 {
    8080
}
