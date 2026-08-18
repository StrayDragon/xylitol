//! AppConfig type definitions with serde + schemars derives.

use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::error::LoadError;

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

    // ── always compiled ──────────────────────────────────────────────
    pub hooks: HooksConfig,
    pub security: SecurityConfig,
    pub tools: ToolsConfig,

    // ── always compiled ────────────────────────────────────────────────
    pub session: Option<SessionConfig>,
    /// YAML loading-phase compaction config. Mapped to runtime
    /// `CompactionSettings` (in `agent::compaction::settings`)
    /// via `From<XyCompactionSettingsConfig>`.
    /// Schema twin: [`CompactionSettingsSchema`] (domain type is serde-only).
    #[schemars(with = "Option<CompactionSettingsSchema>")]
    pub compaction: Option<crate::protocol::compaction_config::XyCompactionSettingsConfig>,

    pub mcp_servers: Option<Vec<McpServerConfig>>,

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

    /// Same-turn tool batch scheduling (c1545). Default sequential.
    #[serde(default)]
    pub tool_batch: ToolBatchConfig,
}

/// Product TUI knobs under top-level `tui:` (c1560 / c1761).
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(default)]
pub struct TuiConfig {
    /// How many prior same-cwd sessions seed ↑/↓ send history on a pure new session.
    #[serde(default = "default_editor_history_seed_sessions")]
    pub editor_history_seed_sessions: u32,
    /// Nested ActivityFold (envelope / cluster / live window).
    #[serde(default)]
    pub activity_fold: TuiActivityFoldConfig,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            editor_history_seed_sessions: default_editor_history_seed_sessions(),
            activity_fold: TuiActivityFoldConfig::default(),
        }
    }
}

fn default_editor_history_seed_sessions() -> u32 {
    1
}

fn default_true() -> bool {
    true
}

fn default_keep_recent_turns() -> u32 {
    2
}

/// `tui.activity_fold` (c1761 / rc28).
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct TuiActivityFoldConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_keep_recent_turns")]
    pub keep_recent_turns: u32,
    #[serde(default)]
    pub stream_collapse: ActivityFoldStreamCollapse,
    #[serde(default = "default_true")]
    pub auto_on_rebuild: bool,
    #[serde(default = "default_true")]
    pub auto_on_turn_end: bool,
}

impl Default for TuiActivityFoldConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            keep_recent_turns: 2,
            stream_collapse: ActivityFoldStreamCollapse::Envelope,
            auto_on_rebuild: true,
            auto_on_turn_end: true,
        }
    }
}

/// How ended / distant turns collapse (`tui.activity_fold.stream_collapse`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActivityFoldStreamCollapse {
    #[default]
    Envelope,
    Clusters,
}

/// `[tool_batch]` — same-turn tool call scheduling (c1545 / rc26).
///
/// Only `mode` is accepted. Extra fields (e.g. MCP allowlists) fail load.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct ToolBatchConfig {
    #[serde(default)]
    pub mode: ToolBatchMode,
}

/// `tool_batch.mode` wire values.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ToolBatchMode {
    Sequential,
    #[default]
    BarrierParallel,
}

impl From<ToolBatchMode> for crate::protocol::ports::XyBatchMode {
    fn from(value: ToolBatchMode) -> Self {
        match value {
            ToolBatchMode::Sequential => Self::Sequential,
            ToolBatchMode::BarrierParallel => Self::BarrierParallel,
        }
    }
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
/// Split from agent-level [`XyModelConfig`](crate::protocol::model::XyModelConfig) —
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
/// References [`XyModelKind`](crate::protocol::model::XyModelKind) for the provider;
/// the kind's serde representation is the YAML wire format. Schema uses a string
/// twin so protocol stays free of schemars (c510).
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct ModelEntry {
    #[schemars(with = "String")]
    pub provider: crate::protocol::model::XyModelKind,
    pub model: String,
    /// Optional custom base URL for OpenAI-compatible or Anthropic-compatible APIs.
    #[serde(default)]
    pub base_url: Option<String>,
    /// Optional adapter API type: `openai-responses` | `openai-completions` | `anthropic-messages`.
    #[serde(default)]
    pub api: Option<String>,
    /// Named wire/thinking dialect (`generic` | `deepseek`). Omit → generic WirePolicy.
    /// Free-form `extra_policy` YAML is not accepted — profiles live in bridge.
    #[serde(default)]
    pub compat: Option<String>,
    /// Optional per-model API key (supports `{{ secret.* }}` after config render).
    /// Omit or empty → empty key at register time; MUST NOT fall back to kind-level
    /// `OPENAI_API_KEY` / `ANTHROPIC_API_KEY` (c2010 / m17).
    #[serde(default)]
    pub api_key: Option<String>,
    /// Optional fallback model ID (must be another key in `models`).
    #[serde(default)]
    pub fallback: Option<String>,
    /// Whether this model supports thinking/reasoning. Default: true for all.
    #[serde(default = "default_thinking")]
    pub thinking: bool,
    /// Ordered, vendor-declared thinking level names. Empty tokens fail load.
    #[serde(default)]
    pub thinking_levels: Option<Vec<String>>,
    /// Optional level → provider effort/budget string map (`null` value = omit that level).
    /// Keys must be declared by this model's `thinking_levels` support list.
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

impl ModelEntry {
    /// Resolve and validate this model's freeform thinking configuration once.
    pub fn resolve_thinking_config(
        &self,
    ) -> Result<(Vec<String>, crate::protocol::model::ThinkingLevelMap), LoadError> {
        let levels = crate::protocol::model::resolve_configured_levels(
            self.thinking,
            self.thinking_levels.as_deref(),
        )?;
        let map = self.thinking_level_map.clone().unwrap_or_default();
        crate::protocol::model::validate_thinking_level_map(&map, &levels)?;
        Ok((levels, map))
    }
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
) -> Result<xylitol_ai_bridge::registry::TokenizerOverride, LoadError> {
    use xylitol_ai_bridge::registry::TokenizerOverride;
    let s = raw.trim();
    if s.is_empty() {
        return Err(LoadError::validation("tokenizer ref is empty"));
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
            .ok_or_else(|| LoadError::validation(format!("tokenizers.{s}: need repo or path")))?;
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
    Err(LoadError::validation(format!(
        "unknown tokenizer `{s}`: use a name from tokenizers:, HF owner/repo, path, or builtin"
    )))
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
    /// Validate freeform thinking lists and maps for every model entry.
    pub fn validate_thinking_levels(&self) -> Result<(), LoadError> {
        for (alias, entry) in &self.model.models {
            entry
                .resolve_thinking_config()
                .map_err(|e| LoadError::validation(format!("models.{alias}: {e}")))?;
        }
        Ok(())
    }

    /// `session.max_turns` must be absent or a positive integer (c1620 / rc27).
    pub fn validate_session_max_turns(&self) -> Result<(), LoadError> {
        match self.session.as_ref().and_then(|s| s.max_turns) {
            None => Ok(()),
            Some(0) => Err(LoadError::validation(
                "session.max_turns must be a positive integer (got 0)",
            )),
            Some(_) => Ok(()),
        }
    }

    /// Soft-check tokenizer refs (pre-1.0: warn via Err only for clearly broken named refs).
    pub fn validate_model_tokenizers(&self) -> Result<(), LoadError> {
        for (alias, entry) in &self.model.models {
            if let Some(raw) = &entry.tokenizer {
                resolve_tokenizer_ref(&self.tokenizers, raw)
                    .map_err(|e| LoadError::validation(format!("models.{alias}.tokenizer: {e}")))?;
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

    /// Resolve a model alias to a runtime [`XyModelConfig`](crate::protocol::model::XyModelConfig).
    ///
    /// Only explicit `models.models` aliases resolve. Omitted/`""` `api_key` → empty string;
    /// MUST NOT fall back to kind-level env (c2010 / m17).
    pub fn resolve_model(
        &self,
        model_id: &str,
    ) -> Result<crate::protocol::model::XyModelConfig, LoadError> {
        use crate::protocol::model::XyModelConfig;

        let entry = self.model.models.get(model_id).ok_or_else(|| {
            LoadError::validation(format!(
                "unknown model alias `{model_id}`: add it under models.models in config.yaml"
            ))
        })?;

        let api_key = match &entry.api_key {
            Some(k) if !k.is_empty() => k.clone(),
            // Explicit empty or omitted → empty (no kind-env fallback; c2010 / m17).
            Some(_) | None => String::new(),
        };

        Ok(XyModelConfig {
            kind: entry.provider,
            api_key,
            model: entry.model.clone(),
            base_url: entry.base_url.clone(),
            api: entry.api.clone(),
            compat: entry.compat.clone(),
        })
    }

    /// Resolve a model alias to [`XyModelMeta`](crate::protocol::model::XyModelMeta) for the registry.
    ///
    /// Composes [`resolve_model`](Self::resolve_model) with per‑model metadata (thinking support,
    /// context window size) from [`ModelEntry`] or sensible defaults.
    pub fn resolve_model_meta(
        &self,
        model_id: &str,
    ) -> Result<crate::protocol::model::XyModelMeta, LoadError> {
        use crate::protocol::model::XyModelMeta;
        use crate::protocol::model::default_context_window_for;

        let model_config = self.resolve_model(model_id)?;
        let entry = self.model.models.get(model_id);

        let thinking = entry.map(|e| e.thinking).unwrap_or(true);
        let context_window = entry
            .and_then(|e| (e.context_window > 0).then_some(e.context_window))
            .unwrap_or_else(|| default_context_window_for(model_config.kind));

        let (thinking_levels, thinking_level_map) = match entry {
            Some(entry) => entry.resolve_thinking_config()?,
            None => (
                vec![crate::protocol::model::THINKING_OFF.into()],
                Default::default(),
            ),
        };

        Ok(XyModelMeta {
            id: model_id.to_string(),
            config: model_config.clone(),
            display_name: model_id.to_string(),
            thinking,
            context_window,
            api: model_config.api.clone().unwrap_or_default(),
            provider: model_config.kind.provider_name().to_string(),
            cost_input: 0.0,
            cost_output: 0.0,
            cost_cache_read: 0.0,
            cost_cache_write: 0.0,
            max_tokens: 0,
            thinking_levels,
            thinking_level_map,
        })
    }

    /// Resolve a named agent profile to a [`crate::protocol::model::ResolvedProfile`].
    pub fn resolve_profile(
        &self,
        name: &str,
    ) -> Result<crate::protocol::model::ResolvedProfile, LoadError> {
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
                LoadError::validation(
                    "no model configured: set `--model`, `execution.model`, `model.default_model`, \
                     or `agents.profiles.<name>.model`"
                        .to_string(),
                )
            })?;
        let model_config = self.resolve_model(model_id)?;

        Ok(crate::protocol::model::ResolvedProfile {
            model_config,
            system_prompt,
            allowed_tools,
            name: name.into(),
        })
    }

    /// Resolve the default agent profile.
    pub fn resolve_default_profile(
        &self,
    ) -> Result<crate::protocol::model::ResolvedProfile, LoadError> {
        let name = if self.agents.default_profile.is_empty() {
            "default"
        } else {
            &self.agents.default_profile
        };
        self.resolve_profile(name)
    }
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
    /// Backend selection. Only `"glob"` (app-level pattern matching) is
    /// delivered. Default: `"glob"`.
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
    /// Optional ReAct turn cap (c1620). When set, bootstrap installs
    /// `should_stop_after_turn` so the run ends after N completed turns.
    /// Absent / null = no turn-count stop hook (product default).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_turns: Option<u32>,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            auto_snapshot: false,
            max_snapshots: default_max_snapshots(),
            storage: SessionStorageConfig::default(),
            max_turns: None,
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
// MCP
// ---------------------------------------------------------------------------

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
    pub fn validate(&self) -> Result<(), LoadError> {
        if self.name.trim().is_empty() {
            return Err(LoadError::validation("mcp server name must not be empty"));
        }
        match self.transport {
            McpTransportKind::Stdio => match self.command.as_deref().map(str::trim) {
                None | Some("") => Err(LoadError::validation(
                    "command is required for stdio transport",
                )),
                Some(_) => Ok(()),
            },
            McpTransportKind::Sse => match self.url.as_deref().map(str::trim) {
                None | Some("") => Err(LoadError::validation("url is required for sse transport")),
                Some(u) if !(u.starts_with("http://") || u.starts_with("https://")) => Err(
                    LoadError::validation(format!("url must be http(s) for sse transport: {u}")),
                ),
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
mod session_max_turns_tests {
    use super::*;

    #[test]
    fn session_max_turns_deserializes() {
        let cfg: AppConfig = yaml_serde::from_str(
            r#"
session:
  storage: {}
  max_turns: 12
"#,
        )
        .expect("valid session.max_turns");
        assert_eq!(cfg.session.as_ref().and_then(|s| s.max_turns), Some(12));
        cfg.validate_session_max_turns().expect("12 is valid");
    }

    #[test]
    fn session_max_turns_zero_fails_validation() {
        let cfg: AppConfig = yaml_serde::from_str(
            r#"
session:
  storage: {}
  max_turns: 0
"#,
        )
        .expect("0 deserializes as u32");
        let err = cfg
            .validate_session_max_turns()
            .expect_err("0 must fail validation");
        assert!(err.to_string().contains("max_turns"), "{err}");
    }

    #[test]
    fn session_max_turns_absent_is_ok() {
        let cfg = AppConfig::default();
        cfg.validate_session_max_turns().expect("absent is ok");
        assert!(cfg.session.as_ref().and_then(|s| s.max_turns).is_none());
    }
}

#[cfg(test)]
mod thinking_levels_tests {
    use super::*;
    use crate::protocol::model::XyModelKind;

    fn fake_entry(thinking: bool, levels: Option<Vec<&str>>) -> ModelEntry {
        ModelEntry {
            provider: XyModelKind::Fake,
            model: "fake-1".into(),
            base_url: None,
            api: None,
            compat: None,
            api_key: None,
            fallback: None,
            thinking,
            thinking_levels: levels.map(|v| v.into_iter().map(str::to_string).collect()),
            thinking_level_map: None,
            context_window: 0,
            tokenizer: None,
        }
    }

    #[test]
    fn resolve_model_meta_default_levels_are_off_only() {
        let mut cfg = AppConfig::default();
        cfg.model.models.insert("f".into(), fake_entry(true, None));
        let meta = cfg.resolve_model_meta("f").unwrap();
        assert_eq!(meta.thinking_levels, vec!["off".to_string()]);
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
    fn validate_thinking_levels_accepts_freeform_names() {
        let mut cfg = AppConfig::default();
        cfg.model
            .models
            .insert("f".into(), fake_entry(true, Some(vec!["bogon"])));
        cfg.validate_thinking_levels().unwrap();
    }

    #[test]
    fn resolve_model_meta_thinking_level_map() {
        let mut cfg = AppConfig::default();
        let mut entry = fake_entry(true, Some(vec!["off", "high"]));
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
    fn validate_thinking_level_map_rejects_key_outside_declared_list() {
        let mut cfg = AppConfig::default();
        let mut entry = fake_entry(true, Some(vec!["off", "high"]));
        let mut map = std::collections::HashMap::new();
        map.insert("max".into(), Some("x".into()));
        entry.thinking_level_map = Some(map);
        cfg.model.models.insert("f".into(), entry);
        let err = cfg.validate_thinking_levels().unwrap_err();
        assert!(err.to_string().contains("max"));
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
  endpoint: "http://127.0.0.1:3000/api/public/otel"
  protocol: http-binary
  environment: dev
  service_name: xylitol
"#,
        )
        .expect("otel section");
        assert_eq!(cfg.otel.exporter, OtelExporterKind::OtlpHttp);
        assert_eq!(
            cfg.otel.endpoint.as_deref(),
            Some("http://127.0.0.1:3000/api/public/otel")
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

    #[test]
    fn tui_activity_fold_defaults() {
        let cfg: AppConfig = yaml_serde::from_str("models: {}").expect("minimal");
        assert!(cfg.tui.activity_fold.enabled);
        assert_eq!(cfg.tui.activity_fold.keep_recent_turns, 2);
        assert_eq!(
            cfg.tui.activity_fold.stream_collapse,
            ActivityFoldStreamCollapse::Envelope
        );
        assert!(cfg.tui.activity_fold.auto_on_rebuild);
        assert!(cfg.tui.activity_fold.auto_on_turn_end);
    }

    #[test]
    fn tui_activity_fold_parses_clusters() {
        let cfg: AppConfig = yaml_serde::from_str(
            r#"
models: {}
tui:
  activity_fold:
    enabled: false
    keep_recent_turns: 4
    stream_collapse: clusters
    auto_on_rebuild: false
    auto_on_turn_end: false
"#,
        )
        .expect("activity_fold");
        assert!(!cfg.tui.activity_fold.enabled);
        assert_eq!(cfg.tui.activity_fold.keep_recent_turns, 4);
        assert_eq!(
            cfg.tui.activity_fold.stream_collapse,
            ActivityFoldStreamCollapse::Clusters
        );
        assert!(!cfg.tui.activity_fold.auto_on_rebuild);
        assert!(!cfg.tui.activity_fold.auto_on_turn_end);
    }

    #[test]
    fn tui_activity_fold_invalid_stream_collapse_fails() {
        let err = yaml_serde::from_str::<AppConfig>(
            r#"
models: {}
tui:
  activity_fold:
    stream_collapse: envelope_please
"#,
        );
        assert!(err.is_err(), "illegal stream_collapse must fail load");
    }

    #[test]
    fn tool_batch_default_barrier_parallel() {
        let cfg: AppConfig = yaml_serde::from_str("models: {}").expect("minimal");
        assert_eq!(cfg.tool_batch.mode, ToolBatchMode::BarrierParallel);
        assert_eq!(
            crate::protocol::ports::XyBatchMode::from(cfg.tool_batch.mode),
            crate::protocol::ports::XyBatchMode::BarrierParallel
        );
    }

    #[test]
    fn tool_batch_sequential_parses() {
        let cfg: AppConfig = yaml_serde::from_str(
            r#"
models: {}
tool_batch:
  mode: sequential
"#,
        )
        .expect("tool_batch sequential");
        assert_eq!(cfg.tool_batch.mode, ToolBatchMode::Sequential);
    }

    #[test]
    fn tool_batch_barrier_parallel_parses() {
        let cfg: AppConfig = yaml_serde::from_str(
            r#"
models: {}
tool_batch:
  mode: barrier_parallel
"#,
        )
        .expect("tool_batch section");
        assert_eq!(cfg.tool_batch.mode, ToolBatchMode::BarrierParallel);
    }

    #[test]
    fn tool_batch_invalid_mode_fails() {
        let err = yaml_serde::from_str::<AppConfig>(
            r#"
models: {}
tool_batch:
  mode: parallel
"#,
        );
        assert!(err.is_err(), "invalid mode must fail load");
    }

    #[test]
    fn tool_batch_rejects_patterns_field() {
        let err = yaml_serde::from_str::<AppConfig>(
            r#"
models: {}
tool_batch:
  mode: sequential
  parallel_safe_patterns: ["mcp:*"]
"#,
        );
        assert!(
            err.is_err(),
            "MCP allowlist / patterns fields must not be accepted"
        );
    }
}
