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
    pub compaction: Option<crate::protocol::compaction_config::XyCompactionSettingsConfig>,

    pub skills: Option<Vec<SkillConfig>>,
    pub mcp_servers: Option<Vec<McpServerConfig>>,

    pub review: Option<ReviewConfig>,

    /// Named tokenizer sources shared by models (c1380; pre-1.0 simple shape).
    #[serde(default)]
    pub tokenizers: HashMap<String, TokenizerEntry>,

    /// Context token estimate gates (c1420).
    #[serde(default)]
    pub token_estimate: TokenEstimateConfig,

    /// Optional remote OTLP export (c1475). Default exporter=none (no remote traffic).
    #[serde(default)]
    pub otel: OtelConfig,

    /// Product TUI surface settings (c1560).
    #[serde(default)]
    pub tui: TuiConfig,
}

/// Product TUI knobs under top-level `tui:` (c1560).
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(default)]
pub struct TuiConfig {
    /// How many prior same-cwd sessions seed ↑/↓ send history on a pure new session.
    #[serde(default = "default_editor_history_seed_sessions")]
    pub editor_history_seed_sessions: u32,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            editor_history_seed_sessions: default_editor_history_seed_sessions(),
        }
    }
}

fn default_editor_history_seed_sessions() -> u32 {
    1
}

/// Remote OpenTelemetry export settings (`[otel]`). Orthogonal to local file
/// provider-trace gates (`XYLITOL_PROVIDER_TRACE` / debug build).
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(default)]
pub struct OtelConfig {
    /// `none` (default) or `otlp-http`.
    #[serde(default)]
    pub exporter: OtelExporterKind,
    /// OTLP/HTTP base or traces endpoint (e.g. `http://host:3000/api/public/otel`).
    #[serde(default)]
    pub endpoint: Option<String>,
    /// Wire encoding for OTLP/HTTP.
    #[serde(default)]
    pub protocol: OtelHttpProtocol,
    /// Resource `deployment.environment` / Langfuse environment hint.
    #[serde(default)]
    pub environment: Option<String>,
    /// Resource `service.name` (default: `xylitol`).
    #[serde(default)]
    pub service_name: Option<String>,
    /// Extra OTLP HTTP headers (e.g. `Authorization`). Prefer `{{ secret.* }}` /
    /// env; Langfuse Basic Auth MAY also be derived from `LANGFUSE_*` env.
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Whether generation spans attach observation input/output (c1485).
    /// Default `none` — metadata/usage only. Also gates `agent.turn` root
    /// user-prompt preview for Langfuse Session list (c1555).
    #[serde(default)]
    pub observation_io: OtelObservationIo,
    /// Whether `tool.execute` spans attach args/result observation I/O (c1550).
    /// Separate from [`Self::observation_io`] — tool payloads often differ in sensitivity.
    /// Default `none`.
    #[serde(default)]
    pub tool_observation_io: OtelObservationIo,
    /// OTLP/HTTP client timeout seconds (default 60). Large batches over LAN
    /// often exceed the previous hard-coded 10s and surface as generic
    /// `network error` even when Langfuse itself is healthy.
    #[serde(default = "default_otel_export_timeout_secs")]
    pub export_timeout_secs: u64,
}

fn default_otel_export_timeout_secs() -> u64 {
    60
}

impl Default for OtelConfig {
    fn default() -> Self {
        Self {
            exporter: OtelExporterKind::default(),
            endpoint: None,
            protocol: OtelHttpProtocol::default(),
            environment: None,
            service_name: None,
            headers: HashMap::new(),
            observation_io: OtelObservationIo::default(),
            tool_observation_io: OtelObservationIo::default(),
            export_timeout_secs: default_otel_export_timeout_secs(),
        }
    }
}

/// `[otel].observation_io` / `tool_observation_io` — observation I/O on Langfuse.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum OtelObservationIo {
    #[default]
    None,
    Truncated,
    Full,
}

/// `[otel].exporter` — remote export off by default.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum OtelExporterKind {
    #[default]
    None,
    OtlpHttp,
}

/// `[otel].protocol` for OTLP/HTTP.
///
/// Default is JSON: Langfuse self-host (v3.x) reliably ingests OTLP/HTTP JSON;
/// protobuf binary has been observed to flush without creating traces.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum OtelHttpProtocol {
    HttpBinary,
    #[default]
    HttpJson,
}

impl OtelConfig {
    /// Effective remote export enabled (config only; feature gate is separate).
    pub fn wants_otlp_http(&self) -> bool {
        matches!(self.exporter, OtelExporterKind::OtlpHttp)
    }
}

/// Gates for multi-source context token estimation (c1420 / paa10 / rc19).
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct TokenEstimateConfig {
    /// Local tokenizer encode: only `on` | `off` (default off).
    #[serde(default)]
    pub local_tokenizer: LocalTokenizerGate,
}

/// `token_estimate.local_tokenizer` — on/off only (no every-N / idle).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum LocalTokenizerGate {
    On,
    #[default]
    Off,
}

impl LocalTokenizerGate {
    pub fn is_on(&self) -> bool {
        matches!(self, Self::On)
    }
}

// ---------------------------------------------------------------------------
// Model
// ---------------------------------------------------------------------------

/// Top-level model configuration section.
/// Split from agent-level [`XyModelConfig`](crate::protocol::model_config::XyModelConfig) —
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
/// References [`XyModelKind`](crate::protocol::model_config::XyModelKind) for the provider;
/// the kind's serde representation is the YAML wire format. Schema uses a string
/// twin so domain stays free of schemars (c510).
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct ModelEntry {
    #[schemars(with = "String")]
    pub provider: crate::protocol::model_config::XyModelKind,
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
    /// Optional explicit thinking levels (may contain holes). Unknown names fail load.
    #[serde(default)]
    pub thinking_levels: Option<Vec<String>>,
    /// Optional level → provider effort/budget string map (`null` value = omit that level).
    /// Unknown keys fail load. Missing keys use adapter built-in defaults at request time.
    #[serde(default)]
    pub thinking_level_map: Option<std::collections::HashMap<String, Option<String>>>,
    /// Context window size in tokens. Default: 0 (auto-detect from provider).
    #[serde(default)]
    pub context_window: u64,
    /// Tokenizer ref: named entry in top-level `tokenizers`, HF `owner/repo`,
    /// local path, or `builtin` (c1380; pre-1.0 string-only).
    #[serde(default)]
    pub tokenizer: Option<String>,
}

/// Shared tokenizer definition under top-level `tokenizers:` (c1380).
///
/// Prefer `repo` (HF). If `path` is set, load local file and ignore `repo`.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct TokenizerEntry {
    /// HuggingFace repo id, e.g. `Qwen/Qwen3.6-35B-A3B`.
    #[serde(default)]
    pub repo: Option<String>,
    /// File within the repo. Default `tokenizer.json`.
    #[serde(default = "default_tokenizer_file")]
    pub file: String,
    /// Local filesystem path to a tokenizer.json (wins over `repo` when set).
    #[serde(default)]
    pub path: Option<String>,
}

fn default_tokenizer_file() -> String {
    "tokenizer.json".into()
}

/// Resolve a model alias's `tokenizer:` string against `tokenizers:` table.
///
/// Pre-1.0: keep it dumb — named ref, `builtin`, path-ish, or HF `owner/repo`.
pub fn resolve_tokenizer_ref(
    tokenizers: &HashMap<String, TokenizerEntry>,
    raw: &str,
) -> Result<xylitol_ai_bridge::registry::TokenizerOverride, String> {
    use xylitol_ai_bridge::registry::TokenizerOverride;
    let s = raw.trim();
    if s.is_empty() {
        return Err("tokenizer ref is empty".into());
    }
    if s.eq_ignore_ascii_case("builtin") {
        return Ok(TokenizerOverride::Builtin);
    }
    if let Some(entry) = tokenizers.get(s) {
        if let Some(path) = entry
            .path
            .as_ref()
            .map(|p| p.trim())
            .filter(|p| !p.is_empty())
        {
            return Ok(TokenizerOverride::Local {
                path: std::path::PathBuf::from(path),
            });
        }
        let repo = entry
            .repo
            .as_ref()
            .map(|r| r.trim())
            .filter(|r| !r.is_empty())
            .ok_or_else(|| format!("tokenizers.{s}: need repo or path"))?;
        let file = if entry.file.trim().is_empty() {
            "tokenizer.json".into()
        } else {
            entry.file.clone()
        };
        return Ok(TokenizerOverride::HuggingFace {
            repo: repo.to_string(),
            file,
        });
    }
    // Local path heuristics (pre-1.0 messy OK).
    if s.starts_with('/')
        || s.starts_with('.')
        || s.ends_with(".json")
        || s.contains('\\')
        || std::path::Path::new(s).exists()
    {
        return Ok(TokenizerOverride::Local {
            path: std::path::PathBuf::from(s),
        });
    }
    // HF repo: contains '/'
    if s.contains('/') {
        return Ok(TokenizerOverride::HuggingFace {
            repo: s.to_string(),
            file: "tokenizer.json".into(),
        });
    }
    Err(format!(
        "unknown tokenizer `{s}`: use a name from tokenizers:, HF owner/repo, path, or builtin"
    ))
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

/// Agent profile — binds model, prompt, tools.
///
/// Flat struct to support YAML anchor/alias merge keys:
/// ```yaml
/// agents:
///   profiles:
///     default: &default-agent
///       model: gpt-4o
///     planning:
///       <<: *default-agent
///       model: claude-opus
///       system_prompt: "You are a planning agent."
/// ```
///
/// Unknown fields (including removed `max_iterations`) are rejected.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
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

impl AppConfig {
    /// Validate optional `thinking_levels` on every model entry (unknown names fail).
    pub fn validate_thinking_levels(&self) -> Result<(), String> {
        for (alias, entry) in &self.model.models {
            crate::protocol::types::ThinkingLevel::resolve_configured_levels(
                entry.thinking,
                entry.thinking_levels.as_deref(),
            )
            .map_err(|e| format!("models.{alias}: {e}"))?;
            if let Some(map) = &entry.thinking_level_map {
                crate::protocol::types::validate_thinking_level_map(map)
                    .map_err(|e| format!("models.{alias}: {e}"))?;
            }
        }
        Ok(())
    }

    /// Soft-check tokenizer refs (pre-1.0: warn via Err only for clearly broken named refs).
    pub fn validate_model_tokenizers(&self) -> Result<(), String> {
        for (alias, entry) in &self.model.models {
            if let Some(raw) = &entry.tokenizer {
                resolve_tokenizer_ref(&self.tokenizers, raw)
                    .map_err(|e| format!("models.{alias}.tokenizer: {e}"))?;
            }
        }
        Ok(())
    }

    /// Resolve `models.<id>.tokenizer` to a bridge override, if set.
    pub fn tokenizer_override_for(
        &self,
        model_alias: &str,
    ) -> Option<xylitol_ai_bridge::registry::TokenizerOverride> {
        let raw = self.model.models.get(model_alias)?.tokenizer.as_ref()?;
        resolve_tokenizer_ref(&self.tokenizers, raw).ok()
    }

    /// Resolve a model alias to a runtime [`XyModelConfig`](crate::protocol::model_config::XyModelConfig).
    pub fn resolve_model(
        &self,
        model_id: &str,
    ) -> Result<crate::protocol::model_config::XyModelConfig, String> {
        use crate::protocol::model_config::{XyModelConfig, XyModelKind};

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

    /// Resolve a model alias to [`XyModelMeta`](crate::protocol::types::XyModelMeta) for the registry.
    ///
    /// Composes [`resolve_model`](Self::resolve_model) with per‑model metadata (thinking support,
    /// context window size) from [`ModelEntry`] or sensible defaults.
    pub fn resolve_model_meta(
        &self,
        model_id: &str,
    ) -> Result<crate::protocol::types::XyModelMeta, String> {
        use crate::protocol::model_config::default_context_window_for;
        use crate::protocol::types::XyModelMeta;

        let model_config = self.resolve_model(model_id)?;
        let entry = self.model.models.get(model_id);

        let thinking = entry.map(|e| e.thinking).unwrap_or(true);
        let context_window = entry
            .and_then(|e| (e.context_window > 0).then_some(e.context_window))
            .unwrap_or_else(|| default_context_window_for(model_config.kind));

        let levels = crate::protocol::types::ThinkingLevel::resolve_configured_levels(
            thinking,
            entry.and_then(|e| e.thinking_levels.as_deref()),
        )?;
        let thinking_levels = levels.iter().map(|l| l.as_str().to_string()).collect();
        let thinking_level_map = entry
            .and_then(|e| e.thinking_level_map.clone())
            .unwrap_or_default();

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
            thinking_levels,
            thinking_level_map,
        })
    }

    /// Resolve a named agent profile to a [`crate::protocol::model_config::ResolvedProfile`].
    pub fn resolve_profile(
        &self,
        name: &str,
    ) -> Result<crate::protocol::model_config::ResolvedProfile, String> {
        let profile = self.agents.profiles.get(name);

        let (model_ref, system_prompt, allowed_tools) = match profile {
            Some(p) => (
                p.model.as_deref(),
                p.system_prompt.as_ref().cloned(),
                p.allowed_tools.as_ref().cloned(),
            ),
            None => (None, self.execution.system_prompt.clone(), None),
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

        Ok(crate::protocol::model_config::ResolvedProfile {
            model_config,
            system_prompt,
            allowed_tools,
            name: name.into(),
        })
    }

    /// Resolve the default agent profile.
    pub fn resolve_default_profile(
        &self,
    ) -> Result<crate::protocol::model_config::ResolvedProfile, String> {
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

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct HookEntry {
    /// Shell command to execute (e.g., "python3 /path/to/hook.py").
    pub command: String,
    /// Event patterns to match (e.g., "pre.tool_call", "post.step_complete").
    /// Supports exact and prefix matching.
    #[serde(default)]
    pub events: Vec<String>,
    /// Optional timeout in seconds. Omit / null = unlimited.
    #[serde(default)]
    pub timeout_secs: Option<u64>,
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
pub use crate::protocol::compaction_config::XyCompactionSettingsConfig;

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
    /// SSE / streamable-HTTP endpoint URL (sse transport only).
    #[serde(default)]
    pub url: Option<String>,
    /// Extra environment variables for the child process (stdio transport only).
    #[serde(default)]
    pub env: Option<HashMap<String, String>>,
    /// Extra HTTP headers for SSE / streamable-HTTP (e.g. API keys).
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,
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
            headers: None,
        }
    }
}

impl McpServerConfig {
    /// Validate required fields for the selected transport (c1080 / mcp4).
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("mcp server name must not be empty".into());
        }
        match self.transport {
            McpTransportKind::Stdio => match self.command.as_deref().map(str::trim) {
                None | Some("") => Err("command is required for stdio transport".into()),
                Some(_) => Ok(()),
            },
            McpTransportKind::Sse => match self.url.as_deref().map(str::trim) {
                None | Some("") => Err("url is required for sse transport".into()),
                Some(u) if !(u.starts_with("http://") || u.starts_with("https://")) => {
                    Err(format!("url must be http(s) for sse transport: {u}"))
                }
                Some(_) => Ok(()),
            },
        }
    }
}

fn default_mcp_transport() -> McpTransportKind {
    McpTransportKind::Stdio
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
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

#[cfg(test)]
mod agent_profile_tests {
    use super::*;

    #[test]
    fn residual_max_iterations_is_rejected() {
        let err = yaml_serde::from_str::<AgentProfile>(
            r#"
model: gpt-4o
max_iterations: 50
"#,
        )
        .expect_err("max_iterations must be unknown / denied");
        let msg = err.to_string();
        assert!(
            msg.contains("max_iterations") || msg.contains("unknown field"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn known_profile_fields_deserialize() {
        let p: AgentProfile = yaml_serde::from_str(
            r#"
model: gpt-4o
system_prompt: "hi"
"#,
        )
        .expect("valid profile");
        assert_eq!(p.model.as_deref(), Some("gpt-4o"));
        assert_eq!(p.system_prompt.as_deref(), Some("hi"));
    }
}

#[cfg(test)]
mod thinking_levels_tests {
    use super::*;
    use crate::protocol::model_config::XyModelKind;
    use crate::protocol::types::ThinkingLevel;

    fn fake_entry(thinking: bool, levels: Option<Vec<&str>>) -> ModelEntry {
        ModelEntry {
            provider: XyModelKind::Fake,
            model: "fake-1".into(),
            base_url: None,
            api: None,
            fallback: None,
            thinking,
            thinking_levels: levels.map(|v| v.into_iter().map(str::to_string).collect()),
            thinking_level_map: None,
            context_window: 0,
            tokenizer: None,
        }
    }

    #[test]
    fn resolve_model_meta_default_standard_levels() {
        let mut cfg = AppConfig::default();
        cfg.model.models.insert("f".into(), fake_entry(true, None));
        let meta = cfg.resolve_model_meta("f").unwrap();
        assert_eq!(
            meta.thinking_levels,
            vec![
                "off".to_string(),
                "minimal".to_string(),
                "low".to_string(),
                "medium".to_string(),
                "high".to_string(),
            ]
        );
        assert!(!meta.thinking_levels.iter().any(|l| l == "xhigh"));
    }

    #[test]
    fn resolve_model_meta_explicit_hole() {
        let mut cfg = AppConfig::default();
        cfg.model
            .models
            .insert("f".into(), fake_entry(true, Some(vec!["high", "max"])));
        let meta = cfg.resolve_model_meta("f").unwrap();
        assert_eq!(
            meta.thinking_levels,
            vec!["high".to_string(), "max".to_string()]
        );
    }

    #[test]
    fn resolve_model_meta_thinking_false() {
        let mut cfg = AppConfig::default();
        cfg.model
            .models
            .insert("f".into(), fake_entry(false, Some(vec!["high"])));
        let meta = cfg.resolve_model_meta("f").unwrap();
        assert_eq!(meta.thinking_levels, vec!["off".to_string()]);
    }

    #[test]
    fn validate_thinking_levels_rejects_unknown() {
        let mut cfg = AppConfig::default();
        cfg.model
            .models
            .insert("f".into(), fake_entry(true, Some(vec!["bogon"])));
        let err = cfg.validate_thinking_levels().unwrap_err();
        assert!(err.contains("unknown"));
    }

    #[test]
    fn thinking_level_parse_xhigh() {
        assert_eq!(ThinkingLevel::parse("xhigh"), Some(ThinkingLevel::Xhigh));
    }

    #[test]
    fn resolve_model_meta_thinking_level_map() {
        let mut cfg = AppConfig::default();
        let mut entry = fake_entry(true, None);
        let mut map = std::collections::HashMap::new();
        map.insert("high".into(), Some("max".into()));
        map.insert("off".into(), None);
        entry.thinking_level_map = Some(map);
        cfg.model.models.insert("f".into(), entry);
        let meta = cfg.resolve_model_meta("f").unwrap();
        assert_eq!(
            meta.thinking_level_map
                .get("high")
                .cloned()
                .flatten()
                .as_deref(),
            Some("max")
        );
        assert!(matches!(meta.thinking_level_map.get("off"), Some(None)));
    }

    #[test]
    fn validate_thinking_level_map_rejects_unknown() {
        let mut cfg = AppConfig::default();
        let mut entry = fake_entry(true, None);
        let mut map = std::collections::HashMap::new();
        map.insert("bogon".into(), Some("x".into()));
        entry.thinking_level_map = Some(map);
        cfg.model.models.insert("f".into(), entry);
        let err = cfg.validate_thinking_levels().unwrap_err();
        assert!(err.contains("bogon"));
    }

    #[test]
    fn resolve_model_meta_absent_map_ok() {
        let mut cfg = AppConfig::default();
        cfg.model.models.insert("f".into(), fake_entry(true, None));
        let meta = cfg.resolve_model_meta("f").unwrap();
        assert!(meta.thinking_level_map.is_empty());
    }

    #[test]
    fn tokenizer_ref_named_and_inline() {
        use xylitol_ai_bridge::registry::TokenizerOverride;
        let mut table = HashMap::new();
        table.insert(
            "qwen36".into(),
            TokenizerEntry {
                repo: Some("Qwen/Qwen3.6-35B-A3B".into()),
                file: "tokenizer.json".into(),
                path: None,
            },
        );
        match resolve_tokenizer_ref(&table, "qwen36").unwrap() {
            TokenizerOverride::HuggingFace { repo, file } => {
                assert_eq!(repo, "Qwen/Qwen3.6-35B-A3B");
                assert_eq!(file, "tokenizer.json");
            }
            other => panic!("expected HF, got {other:?}"),
        }
        match resolve_tokenizer_ref(&HashMap::new(), "Qwen/Qwen3.6-35B-A3B").unwrap() {
            TokenizerOverride::HuggingFace { repo, .. } => {
                assert_eq!(repo, "Qwen/Qwen3.6-35B-A3B");
            }
            other => panic!("expected HF, got {other:?}"),
        }
        assert!(matches!(
            resolve_tokenizer_ref(&HashMap::new(), "builtin").unwrap(),
            TokenizerOverride::Builtin
        ));
        assert!(resolve_tokenizer_ref(&HashMap::new(), "nope").is_err());
    }

    #[test]
    fn token_estimate_local_tokenizer_default_off() {
        let cfg: AppConfig = yaml_serde::from_str("models: {}").unwrap();
        assert_eq!(cfg.token_estimate.local_tokenizer, LocalTokenizerGate::Off);
        assert!(!cfg.token_estimate.local_tokenizer.is_on());
    }

    #[test]
    fn token_estimate_local_tokenizer_on() {
        let cfg: AppConfig = yaml_serde::from_str(
            r#"
models: {}
token_estimate:
  local_tokenizer: on
"#,
        )
        .unwrap();
        assert!(cfg.token_estimate.local_tokenizer.is_on());
    }

    #[test]
    fn token_estimate_local_tokenizer_unquoted_off() {
        // YAML 1.1 may treat bare `off` as bool; must still load as Off gate.
        let cfg: AppConfig = yaml_serde::from_str(
            r#"
models: {}
token_estimate:
  local_tokenizer: off
"#,
        )
        .expect("unquoted off must deserialize");
        assert!(!cfg.token_estimate.local_tokenizer.is_on());
        assert_eq!(cfg.token_estimate.local_tokenizer, LocalTokenizerGate::Off);
    }

    #[test]
    fn token_estimate_local_tokenizer_invalid_fails() {
        let err = yaml_serde::from_str::<AppConfig>(
            r#"
models: {}
token_estimate:
  local_tokenizer: every_n
"#,
        );
        assert!(err.is_err());
    }

    #[test]
    fn otel_config_default_is_none() {
        let cfg: AppConfig = yaml_serde::from_str("models: {}").unwrap();
        assert_eq!(cfg.otel.exporter, OtelExporterKind::None);
        assert!(!cfg.otel.wants_otlp_http());
    }

    #[test]
    fn otel_config_otlp_http_parses() {
        let cfg: AppConfig = yaml_serde::from_str(
            r#"
models: {}
otel:
  exporter: otlp-http
  endpoint: "http://coral:3000/api/public/otel"
  protocol: http-binary
  environment: dev
  service_name: xylitol
"#,
        )
        .expect("otel section");
        assert_eq!(cfg.otel.exporter, OtelExporterKind::OtlpHttp);
        assert_eq!(
            cfg.otel.endpoint.as_deref(),
            Some("http://coral:3000/api/public/otel")
        );
        assert_eq!(cfg.otel.protocol, OtelHttpProtocol::HttpBinary);
        assert_eq!(cfg.otel.environment.as_deref(), Some("dev"));
        assert_eq!(cfg.otel.export_timeout_secs, 60);
    }

    #[test]
    fn otel_config_export_timeout_parses() {
        let cfg: AppConfig = yaml_serde::from_str(
            r#"
models: {}
otel:
  exporter: otlp-http
  export_timeout_secs: 90
"#,
        )
        .expect("otel timeout");
        assert_eq!(cfg.otel.export_timeout_secs, 90);
    }

    #[test]
    fn otel_config_observation_io_parses() {
        let cfg: AppConfig = yaml_serde::from_str(
            r#"
models: {}
otel:
  exporter: otlp-http
  observation_io: truncated
  tool_observation_io: full
"#,
        )
        .expect("otel io");
        assert_eq!(cfg.otel.observation_io, OtelObservationIo::Truncated);
        assert_eq!(cfg.otel.tool_observation_io, OtelObservationIo::Full);
    }

    #[test]
    fn otel_config_rejects_unknown_exporter() {
        let err = yaml_serde::from_str::<AppConfig>(
            r#"
models: {}
otel:
  exporter: grpc
"#,
        );
        assert!(err.is_err(), "unknown exporter must fail load");
    }

    #[test]
    fn tui_editor_history_seed_sessions_defaults_to_one() {
        let cfg: AppConfig = yaml_serde::from_str("models: {}").expect("minimal");
        assert_eq!(cfg.tui.editor_history_seed_sessions, 1);
    }

    #[test]
    fn tui_editor_history_seed_sessions_parses() {
        let cfg: AppConfig = yaml_serde::from_str(
            r#"
models: {}
tui:
  editor_history_seed_sessions: 3
"#,
        )
        .expect("tui section");
        assert_eq!(cfg.tui.editor_history_seed_sessions, 3);
    }
}
