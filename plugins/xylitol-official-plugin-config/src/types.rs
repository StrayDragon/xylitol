use std::collections::{BTreeMap, HashMap, HashSet};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::error::LoadError;

pub const CONFIG_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UnknownKeyPolicy {
    #[default]
    Error,
    Warn,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct BootstrapConfig {
    pub schema_version: Option<u32>,
    pub unknown_keys: UnknownKeyPolicy,
    pub plugins: Vec<String>,
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        Self {
            schema_version: None,
            unknown_keys: UnknownKeyPolicy::Error,
            plugins: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AppConfig {
    pub xylitol: BootstrapConfig,
    #[serde(flatten)]
    pub sections: BTreeMap<String, serde_json::Value>,
    #[serde(skip)]
    pub unknown_keys: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ServeLifecycleConfig {
    #[default]
    LastClient,
    Persistent,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ServeIdleTimeout {
    Flag(bool),
    Spec(String),
}

impl Default for ServeIdleTimeout {
    fn default() -> Self {
        Self::Spec("30s".into())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct ServeConfig {
    pub lifecycle: ServeLifecycleConfig,
    pub idle_timeout: ServeIdleTimeout,
}

impl Default for ServeConfig {
    fn default() -> Self {
        Self {
            lifecycle: ServeLifecycleConfig::LastClient,
            idle_timeout: ServeIdleTimeout::default(),
        }
    }
}

impl ServeConfig {
    pub fn autostart_last_client(&self) -> bool {
        if matches!(self.idle_timeout, ServeIdleTimeout::Flag(false)) {
            return false;
        }
        matches!(self.lifecycle, ServeLifecycleConfig::LastClient)
    }

    pub fn idle_duration(&self) -> std::time::Duration {
        match &self.idle_timeout {
            ServeIdleTimeout::Spec(raw) => {
                parse_serve_idle(raw).unwrap_or(std::time::Duration::from_secs(30))
            }
            ServeIdleTimeout::Flag(_) => std::time::Duration::from_secs(30),
        }
    }
}

fn parse_serve_idle(raw: &str) -> Option<std::time::Duration> {
    let raw = raw.trim();
    if let Some(n) = raw.strip_suffix("ms") {
        return n.parse().ok().map(std::time::Duration::from_millis);
    }
    if let Some(n) = raw.strip_suffix('s') {
        return n.parse().ok().map(std::time::Duration::from_secs);
    }
    if let Some(n) = raw.strip_suffix('m') {
        return n
            .parse::<u64>()
            .ok()
            .map(|m| std::time::Duration::from_secs(m.saturating_mul(60)));
    }
    None
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct SubagentConfig {
    pub tools: Vec<String>,
}

impl Default for SubagentConfig {
    fn default() -> Self {
        Self {
            tools: vec![
                "read".into(),
                "grep".into(),
                "find".into(),
                "ls".into(),
                "ffgrep".into(),
                "fffind".into(),
            ],
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct ExternalPluginConfig {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub trusted: bool,
    #[serde(default = "default_external_plugin_timeout_ms")]
    pub timeout_ms: u64,
}

fn default_external_plugin_timeout_ms() -> u64 {
    30_000
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(default)]
pub struct TuiConfig {
    #[serde(default = "default_editor_history_seed_sessions")]
    pub editor_history_seed_sessions: u32,
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
    #[serde(default)]
    pub display_level: ActivityDisplayLevel,
    #[serde(default)]
    pub visible_items: Option<TuiVisibleItems>,
}

impl Default for TuiActivityFoldConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            keep_recent_turns: 2,
            stream_collapse: ActivityFoldStreamCollapse::Envelope,
            auto_on_rebuild: true,
            auto_on_turn_end: true,
            display_level: ActivityDisplayLevel::Summary,
            visible_items: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActivityFoldStreamCollapse {
    #[default]
    Envelope,
    Clusters,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActivityDisplayLevel {
    Quiet,
    #[default]
    Summary,
    Recent,
    Full,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct TuiVisibleItems {
    pub thinking: bool,
    pub tool_header: bool,
    pub tool_output: bool,
    pub diff: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct ToolBatchConfig {
    #[serde(default)]
    pub mode: ToolBatchMode,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ToolBatchMode {
    Sequential,
    #[default]
    BarrierParallel,
}

impl From<ToolBatchMode> for xylitol_plugin_sdk::XyBatchMode {
    fn from(value: ToolBatchMode) -> Self {
        match value {
            ToolBatchMode::Sequential => Self::Sequential,
            ToolBatchMode::BarrierParallel => Self::BarrierParallel,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(default)]
pub struct OtelConfig {
    #[serde(default)]
    pub exporter: OtelExporterKind,
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub protocol: OtelHttpProtocol,
    #[serde(default)]
    pub environment: Option<String>,
    #[serde(default)]
    pub service_name: Option<String>,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub observation_io: OtelObservationIo,
    #[serde(default)]
    pub tool_observation_io: OtelObservationIo,
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

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum OtelObservationIo {
    #[default]
    None,
    Truncated,
    Full,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum OtelExporterKind {
    #[default]
    None,
    OtlpHttp,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum OtelHttpProtocol {
    HttpBinary,
    #[default]
    HttpJson,
}

impl OtelConfig {
    pub fn wants_otlp_http(&self) -> bool {
        matches!(self.exporter, OtelExporterKind::OtlpHttp)
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct ModelsConfig {
    pub default_model: Option<String>,
    pub entries: HashMap<String, ModelEntry>,
}

pub type ModelEntry = xylitol_plugin_sdk::model_entry::XyModelEntryConfig;

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct ExecutionConfig {
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub system_prompt: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AgentProfile {
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub persona: Option<String>,
    #[serde(default)]
    pub tools: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AgentsConfig {
    #[serde(default = "default_profile_name")]
    pub default_profile: String,
    #[serde(default)]
    pub profiles: HashMap<String, AgentProfile>,
}

fn default_profile_name() -> String {
    "default".into()
}

fn compose_profile_prompt(profile: &AgentProfile, fallback: Option<&String>) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(persona) = profile
        .persona
        .as_deref()
        .filter(|text| !text.trim().is_empty())
    {
        parts.push(persona);
    }
    if let Some(prompt) = profile
        .system_prompt
        .as_deref()
        .filter(|text| !text.trim().is_empty())
    {
        parts.push(prompt);
    }
    if parts.is_empty() {
        fallback.cloned()
    } else {
        Some(parts.join("\n\n"))
    }
}

impl AppConfig {
    pub fn section<T: serde::de::DeserializeOwned>(
        &self,
        key: &str,
    ) -> Result<Option<T>, LoadError> {
        self.sections
            .get(key)
            .cloned()
            .map(serde_json::from_value)
            .transpose()
            .map_err(|e| LoadError::validation(format!("config section `{key}`: {e}")))
    }

    pub fn section_or_default<T: serde::de::DeserializeOwned + Default>(
        &self,
        key: &str,
    ) -> Result<T, LoadError> {
        Ok(self.section(key)?.unwrap_or_default())
    }

    pub fn validate_schema_version(&self) -> Result<(), LoadError> {
        match self.xylitol.schema_version {
            Some(version) if version > CONFIG_SCHEMA_VERSION => {
                Err(LoadError::validation(format!(
                    "config schema_version {version} is newer than supported {CONFIG_SCHEMA_VERSION}"
                )))
            }
            _ => Ok(()),
        }
    }

    pub fn validate_unknown_keys(&self, extra_keys: &[String]) -> Result<(), LoadError> {
        let known: HashSet<&str> = super::sections::sections()
            .iter()
            .map(|section| section.key)
            .collect();
        let unknown: Vec<&String> = self
            .unknown_keys
            .iter()
            .filter(|key| !known.contains(key.as_str()))
            .filter(|key| !extra_keys.iter().any(|extra| extra == *key))
            .collect();
        if unknown.is_empty() {
            return Ok(());
        }
        let list = unknown
            .iter()
            .map(|key| format!("`{key}`"))
            .collect::<Vec<_>>()
            .join(", ");
        let message = format!("unknown config keys: {list}");
        match self.xylitol.unknown_keys {
            UnknownKeyPolicy::Error => Err(LoadError::validation(message)),
            UnknownKeyPolicy::Warn => {
                log::warn!(target: "xylitol::config", "{message}");
                Ok(())
            }
        }
    }

    pub fn validate_thinking_levels(&self) -> Result<(), LoadError> {
        let models: ModelsConfig = self.section_or_default("models")?;
        for (alias, entry) in &models.entries {
            entry
                .resolve_thinking_config()
                .map_err(|e| LoadError::validation(format!("models.{alias}: {e}")))?;
        }
        Ok(())
    }

    pub fn validate_session_max_steps_per_turn(&self) -> Result<(), LoadError> {
        let session: Option<SessionConfig> = self.section("session")?;
        match session.as_ref().and_then(|s| s.max_steps_per_turn) {
            None => Ok(()),
            Some(0) => Err(LoadError::validation(
                "session.max_steps_per_turn must be a positive integer (got 0)",
            )),
            Some(_) => Ok(()),
        }
    }

    pub fn validate_session_stage_name(&self) -> Result<(), LoadError> {
        let Some(session) = self.section::<SessionConfig>("session")? else {
            return Ok(());
        };
        let stage = &session.stage_name;
        for (field, value) in [
            ("target_cjk_chars", stage.target_cjk_chars),
            ("target_words", stage.target_words),
            ("max_title_bytes", stage.max_title_bytes),
            ("max_input_bytes", stage.max_input_bytes),
        ] {
            if value == 0 {
                return Err(LoadError::validation(format!(
                    "session.stage_name.{field} must be a positive integer"
                )));
            }
        }
        if stage.max_output_tokens == 0 {
            return Err(LoadError::validation(
                "session.stage_name.max_output_tokens must be a positive integer",
            ));
        }
        if stage.timeout_ms == 0 {
            return Err(LoadError::validation(
                "session.stage_name.timeout_ms must be a positive integer",
            ));
        }
        for (field, value) in [("provider", &stage.provider), ("model", &stage.model)] {
            if value.as_deref().is_some_and(|raw| raw.trim().is_empty()) {
                return Err(LoadError::validation(format!(
                    "session.stage_name.{field} must not be empty"
                )));
            }
        }
        Ok(())
    }

    pub fn validate_trust_audit(&self) -> Result<(), LoadError> {
        let Some(trust) = self.section::<TrustConfig>("trust")? else {
            return Ok(());
        };
        if trust.audit_max_bytes == 0 {
            return Err(LoadError::validation(
                "trust.audit_max_bytes must be a positive integer",
            ));
        }
        if trust.audit_keep_files == 0 {
            return Err(LoadError::validation(
                "trust.audit_keep_files must be at least 1",
            ));
        }
        if trust.audit_retention_days == 0 {
            return Err(LoadError::validation(
                "trust.audit_retention_days must be at least 1",
            ));
        }
        Ok(())
    }

    pub fn validate_tools_selection(&self) -> Result<(), LoadError> {
        let tools: Vec<String> = self.section_or_default("tools")?;
        for (index, raw) in tools.iter().enumerate() {
            let entry = raw.trim();
            if entry.is_empty() {
                return Err(LoadError::validation(format!(
                    "tools[{index}] must not be empty"
                )));
            }
            let name = entry
                .strip_prefix('+')
                .or_else(|| entry.strip_prefix('-'))
                .unwrap_or(entry)
                .trim();
            if name.is_empty() {
                return Err(LoadError::validation(format!(
                    "tools[{index}] must name a tool after `+`/`-`"
                )));
            }
            if name.starts_with('+') || name.starts_with('-') || name.contains(char::is_whitespace)
            {
                return Err(LoadError::validation(format!(
                    "tools[{index}] must be a single tool name (got `{entry}`)"
                )));
            }
        }
        Ok(())
    }

    pub fn resolve_model(
        &self,
        model_id: &str,
    ) -> Result<xylitol_plugin_sdk::model::XyModelConfig, LoadError> {
        let models: ModelsConfig = self.section_or_default("models")?;
        let entry = models.entries.get(model_id).ok_or_else(|| {
            LoadError::validation(format!(
                "unknown model alias `{model_id}`: add it under models.entries in config.yaml"
            ))
        })?;
        Ok(entry.to_model_config())
    }

    pub fn resolve_model_meta(
        &self,
        model_id: &str,
    ) -> Result<xylitol_plugin_sdk::model::XyModelMeta, LoadError> {
        let models: ModelsConfig = self.section_or_default("models")?;
        let entry = models.entries.get(model_id).ok_or_else(|| {
            LoadError::validation(format!(
                "unknown model alias `{model_id}`: add it under models.entries in config.yaml"
            ))
        })?;
        entry.to_model_meta(model_id).map_err(LoadError::validation)
    }

    pub fn resolve_profile(
        &self,
        name: &str,
    ) -> Result<xylitol_plugin_sdk::model::ResolvedProfile, LoadError> {
        let agents: AgentsConfig = self.section_or_default("agents")?;
        let execution: ExecutionConfig = self.section_or_default("execution")?;
        let models: ModelsConfig = self.section_or_default("models")?;
        let profile = agents.profiles.get(name);

        let (model_ref, system_prompt, tools) = match profile {
            Some(p) => (
                p.model.as_deref(),
                compose_profile_prompt(p, execution.system_prompt.as_ref()),
                p.tools.clone(),
            ),
            None => (None, execution.system_prompt.clone(), Vec::new()),
        };

        let model_id = model_ref
            .or(execution.model.as_deref())
            .or(models.default_model.as_deref())
            .ok_or_else(|| {
                LoadError::validation(
                    "no model configured: set `--model`, `execution.model`, `model.default_model`, \
                     or `agents.profiles.<name>.model`"
                        .to_string(),
                )
            })?;
        self.resolve_model(model_id)?;

        Ok(xylitol_plugin_sdk::model::ResolvedProfile {
            model_id: model_id.to_string(),
            system_prompt,
            tools,
        })
    }

    pub fn resolve_default_profile(
        &self,
    ) -> Result<xylitol_plugin_sdk::model::ResolvedProfile, LoadError> {
        let agents: AgentsConfig = self.section_or_default("agents")?;
        let name = if agents.default_profile.is_empty() {
            "default"
        } else {
            &agents.default_profile
        };
        self.resolve_profile(name)
    }
}

pub use xylitol_official_plugin_hooks::{HookEntry, HooksConfig};
pub use xylitol_official_plugin_loop_policies::LoopPoliciesConfig;
pub use xylitol_official_plugin_todo::TodoConfig;
pub use xylitol_official_plugin_workflow::WorkflowConfig;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct TrustConfig {
    #[serde(default = "default_true")]
    pub audit: bool,
    #[serde(default = "default_audit_max_bytes")]
    pub audit_max_bytes: u64,
    #[serde(default = "default_audit_keep_files")]
    pub audit_keep_files: usize,
    #[serde(default = "default_audit_retention_days")]
    pub audit_retention_days: u64,
}

fn default_audit_max_bytes() -> u64 {
    10 * 1024 * 1024
}

fn default_audit_keep_files() -> usize {
    5
}

fn default_audit_retention_days() -> u64 {
    90
}

impl Default for TrustConfig {
    fn default() -> Self {
        Self {
            audit: true,
            audit_max_bytes: default_audit_max_bytes(),
            audit_keep_files: default_audit_keep_files(),
            audit_retention_days: default_audit_retention_days(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct SecurityConfig {
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
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_permission_backend")]
    pub backend: PermissionBackend,
    #[serde(default)]
    pub filesystem: PermissionFilesystemConfig,
    #[serde(default)]
    pub network: PermissionNetworkConfig,
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
    #[default]
    Glob,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct PermissionFilesystemConfig {
    #[serde(default)]
    pub read_allowed: Vec<String>,
    #[serde(default)]
    pub write_allowed: Vec<String>,
    #[serde(default)]
    pub write_denied: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct PermissionNetworkConfig {
    #[serde(default)]
    pub allowed_domains: Vec<String>,
    #[serde(default)]
    pub denied_domains: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct PermissionProcessConfig {
    #[serde(default)]
    pub allowed_paths: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct StageNameConfig {
    pub enabled: bool,
    pub target_cjk_chars: usize,
    pub target_words: usize,
    pub max_title_bytes: usize,
    pub max_input_bytes: usize,
    pub max_output_tokens: u32,
    pub timeout_ms: u64,
    pub provider: Option<String>,
    pub model: Option<String>,
}

impl Default for StageNameConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            target_cjk_chars: 15,
            target_words: 20,
            max_title_bytes: 120,
            max_input_bytes: 8192,
            max_output_tokens: 64,
            timeout_ms: 8000,
            provider: None,
            model: None,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SessionConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_steps_per_turn: Option<u32>,
    #[serde(default)]
    pub stage_name: StageNameConfig,
}

pub use xylitol_official_plugin_dap::DapPluginConfig;
pub use xylitol_official_plugin_lsp::LspPluginConfig;
pub use xylitol_official_plugin_mcp::{
    McpOAuthConfig, McpOAuthKeychain, McpScope, McpServerConfig, McpTransportKind,
};

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct ContextInjectConfig {
    pub global: Vec<ContextInjectorConfig>,
    pub project: Vec<ContextInjectorConfig>,
    pub user: Vec<ContextInjectorConfig>,
}

impl ContextInjectConfig {
    pub fn validate(&self) -> Result<(), String> {
        for tier in [&self.global, &self.project, &self.user] {
            let mut ids = std::collections::HashSet::new();
            for injector in tier {
                if injector.id.trim().is_empty() {
                    return Err("context_inject injector id must not be empty".into());
                }
                if !ids.insert(injector.id.clone()) {
                    return Err(format!(
                        "duplicate context_inject injector id: {}",
                        injector.id
                    ));
                }
                if injector.command.trim().is_empty() {
                    return Err(format!(
                        "context_inject injector {} command must not be empty",
                        injector.id
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn merged(&self) -> Vec<ContextInjectorConfig> {
        let mut merged: Vec<ContextInjectorConfig> = Vec::new();
        for tier in [&self.global, &self.project, &self.user] {
            for injector in tier {
                match merged
                    .iter()
                    .position(|existing| existing.id == injector.id)
                {
                    Some(index) => merged[index] = injector.clone(),
                    None => merged.push(injector.clone()),
                }
            }
        }
        merged
    }

    pub fn is_active(&self) -> bool {
        !self.global.is_empty() || !self.project.is_empty() || !self.user.is_empty()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ContextInjectorConfig {
    pub id: String,
    pub point: ContextInjectPoint,
    pub command: String,
    #[serde(default)]
    pub placement: ContextInjectPlacement,
    #[serde(default)]
    pub dedup: ContextInjectDedup,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
    #[serde(default)]
    pub group: Option<String>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContextInjectPoint {
    SessionStart,
    SessionShutdown,
    TurnStart,
    TurnEnd,
    TurnPre,
    StepStart,
    StepEnd,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContextInjectPlacement {
    #[default]
    HistoryRow,
    Ephemeral,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContextInjectDedup {
    #[default]
    None,
    LatestWins,
    SuppressIfUnchanged,
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

    #[test]
    fn profile_persona_and_tools_deserialize() {
        let p: AgentProfile = yaml_serde::from_str(
            r#"
model: gpt-4o
persona: "You are a reviewer."
system_prompt: "Focus on correctness."
tools: [read, -bash]
"#,
        )
        .expect("profile with persona/tools");
        assert_eq!(p.persona.as_deref(), Some("You are a reviewer."));
        assert_eq!(p.tools, vec!["read".to_string(), "-bash".to_string()]);
    }
}

#[cfg(test)]
mod test_support {
    use super::*;

    pub fn get<T: serde::de::DeserializeOwned + Default>(cfg: &AppConfig, key: &str) -> T {
        cfg.section_or_default(key).expect("config section")
    }

    pub fn set<T: serde::Serialize>(cfg: &mut AppConfig, key: &str, value: T) {
        cfg.sections.insert(
            key.to_string(),
            serde_json::to_value(value).expect("section value"),
        );
    }
}

#[cfg(test)]
mod session_max_steps_per_turn_tests {
    use super::test_support::*;
    use super::*;

    #[test]
    fn session_max_steps_per_turn_deserializes() {
        let cfg: AppConfig = yaml_serde::from_str(
            r#"
session:
  max_steps_per_turn: 12
"#,
        )
        .expect("valid session.max_steps_per_turn");
        assert_eq!(
            get::<Option<SessionConfig>>(&cfg, "session").and_then(|s| s.max_steps_per_turn),
            Some(12)
        );
        cfg.validate_session_max_steps_per_turn()
            .expect("12 is valid");
    }

    #[test]
    fn session_max_steps_per_turn_zero_fails_validation() {
        let cfg: AppConfig = yaml_serde::from_str(
            r#"
session:
  max_steps_per_turn: 0
"#,
        )
        .expect("0 deserializes as u32");
        let err = cfg
            .validate_session_max_steps_per_turn()
            .expect_err("0 must fail validation");
        assert!(err.to_string().contains("max_steps_per_turn"), "{err}");
    }

    #[test]
    fn removed_session_step_cap_key_is_rejected() {
        let removed_key = concat!("max_", "turns");
        let yaml = format!("session:\n  {removed_key}: 7\n");
        let cfg: AppConfig =
            yaml_serde::from_str(&yaml).expect("top-level config keeps sections lazily");
        let err = cfg
            .section::<SessionConfig>("session")
            .expect_err("the removed key must be rejected");
        let msg = err.to_string();
        assert!(
            msg.contains(removed_key) && msg.contains("max_steps_per_turn"),
            "old key must fail and point to the new key: {msg}"
        );
    }

    #[test]
    fn session_max_steps_per_turn_absent_is_ok() {
        let cfg = AppConfig::default();
        cfg.validate_session_max_steps_per_turn()
            .expect("absent is ok");
        assert!(get::<Option<SessionConfig>>(&cfg, "session").is_none());
    }
}

#[cfg(test)]
mod session_stage_name_tests {
    use super::test_support::*;
    use super::*;

    #[test]
    fn stage_name_defaults() {
        let config = StageNameConfig::default();
        assert!(config.enabled);
        assert_eq!(config.target_cjk_chars, 15);
        assert_eq!(config.target_words, 20);
        assert_eq!(config.max_title_bytes, 120);
        assert_eq!(config.max_input_bytes, 8192);
        assert_eq!(config.max_output_tokens, 64);
        assert_eq!(config.timeout_ms, 8000);
        assert!(config.provider.is_none());
        assert!(config.model.is_none());
    }

    #[test]
    fn stage_name_deserializes_and_validates() {
        let cfg: AppConfig = yaml_serde::from_str(
            r#"
session:
  stage_name:
    target_cjk_chars: 12
    max_title_bytes: 96
    provider: local
    model: small
"#,
        )
        .expect("valid stage name");
        let session = get::<Option<SessionConfig>>(&cfg, "session").expect("session");
        assert_eq!(session.stage_name.target_cjk_chars, 12);
        assert_eq!(session.stage_name.max_title_bytes, 96);
        assert_eq!(session.stage_name.provider.as_deref(), Some("local"));
        assert_eq!(session.stage_name.model.as_deref(), Some("small"));
        cfg.validate_session_stage_name().expect("valid");
    }

    #[test]
    fn stage_name_zero_target_fails() {
        let cfg: AppConfig = yaml_serde::from_str(
            r#"
session:
  stage_name:
    target_cjk_chars: 0
"#,
        )
        .expect("zero deserializes");
        let err = cfg
            .validate_session_stage_name()
            .expect_err("zero must fail");
        assert!(err.to_string().contains("target_cjk_chars"), "{err}");
    }

    #[test]
    fn stage_name_empty_provider_fails() {
        let cfg: AppConfig = yaml_serde::from_str(
            r#"
session:
  stage_name:
    provider: "  "
"#,
        )
        .expect("empty provider deserializes");
        let err = cfg
            .validate_session_stage_name()
            .expect_err("empty provider must fail");
        assert!(err.to_string().contains("provider"), "{err}");
    }
}

#[cfg(test)]
mod thinking_levels_tests {
    use super::test_support::*;
    use super::*;
    use xylitol_plugin_sdk::model::XyModelKind;

    fn fake_entry(
        thinking: bool,
        map: Option<std::collections::HashMap<String, Option<String>>>,
    ) -> ModelEntry {
        ModelEntry {
            provider: XyModelKind::Fake,
            model: "fake-1".into(),
            base_url: None,
            api: None,
            compat: None,
            api_key: None,
            fallback: None,
            thinking,
            thinking_level_map: map,
            context_window: 0,
            max_output_tokens: None,
            input_modalities: None,
            prompt_cache: None,
        }
    }

    fn map_of(
        entries: &[(&str, Option<&str>)],
    ) -> std::collections::HashMap<String, Option<String>> {
        entries
            .iter()
            .map(|(level, wire)| ((*level).to_string(), wire.map(str::to_string)))
            .collect()
    }

    #[test]
    fn resolve_model_meta_default_levels_are_off_only() {
        let mut cfg = AppConfig::default();
        let mut models = get::<ModelsConfig>(&cfg, "models");
        models.entries.insert("f".into(), fake_entry(true, None));
        set(&mut cfg, "models", models);
        let meta = cfg.resolve_model_meta("f").unwrap();
        assert_eq!(meta.thinking_levels, vec!["off".to_string()]);
    }

    #[test]
    fn resolve_model_meta_explicit_hole() {
        let mut cfg = AppConfig::default();
        let mut models = get::<ModelsConfig>(&cfg, "models");
        models.entries.insert(
            "f".into(),
            fake_entry(
                true,
                Some(map_of(&[("high", Some("high")), ("max", Some("max"))])),
            ),
        );
        set(&mut cfg, "models", models);
        let meta = cfg.resolve_model_meta("f").unwrap();
        assert_eq!(
            meta.thinking_levels,
            vec!["high".to_string(), "max".to_string()]
        );
    }

    #[test]
    fn resolve_model_meta_thinking_false() {
        let mut cfg = AppConfig::default();
        let mut models = get::<ModelsConfig>(&cfg, "models");
        models.entries.insert(
            "f".into(),
            fake_entry(false, Some(map_of(&[("high", Some("high"))]))),
        );
        set(&mut cfg, "models", models);
        let meta = cfg.resolve_model_meta("f").unwrap();
        assert_eq!(meta.thinking_levels, vec!["off".to_string()]);
    }

    #[test]
    fn validate_thinking_levels_accepts_freeform_names() {
        let mut cfg = AppConfig::default();
        let mut models = get::<ModelsConfig>(&cfg, "models");
        models.entries.insert(
            "f".into(),
            fake_entry(true, Some(map_of(&[("bogon", Some("high"))]))),
        );
        set(&mut cfg, "models", models);
        cfg.validate_thinking_levels().unwrap();
    }

    #[test]
    fn resolve_model_meta_thinking_level_map() {
        let mut cfg = AppConfig::default();
        let entry = fake_entry(true, Some(map_of(&[("off", None), ("high", Some("max"))])));
        let mut models = get::<ModelsConfig>(&cfg, "models");
        models.entries.insert("f".into(), entry);
        set(&mut cfg, "models", models);
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
    fn validate_thinking_levels_rejects_blank_map_key() {
        let mut cfg = AppConfig::default();
        let entry = fake_entry(true, Some(map_of(&[("", None)])));
        let mut models = get::<ModelsConfig>(&cfg, "models");
        models.entries.insert("f".into(), entry);
        set(&mut cfg, "models", models);
        let err = cfg.validate_thinking_levels().unwrap_err();
        assert!(err.to_string().contains("thinking_level_map"));
    }

    #[test]
    fn resolve_model_meta_absent_map_ok() {
        let mut cfg = AppConfig::default();
        let mut models = get::<ModelsConfig>(&cfg, "models");
        models.entries.insert("f".into(), fake_entry(true, None));
        set(&mut cfg, "models", models);
        let meta = cfg.resolve_model_meta("f").unwrap();
        assert!(meta.thinking_level_map.is_empty());
    }

    #[test]
    fn leftover_tokenizer_yaml_keys_are_ignored() {
        let cfg = yaml_serde::from_str::<AppConfig>(
            r#"
models: {}
token_estimate:
  local_tokenizer: on
tokenizers:
  qwen36:
    repo: Qwen/Qwen3.6-35B-A3B
"#,
        );
        assert!(
            cfg.is_ok(),
            "retired tokenizer keys must not fail load: {cfg:?}"
        );
    }

    #[test]
    fn otel_config_default_is_none() {
        let cfg: AppConfig = yaml_serde::from_str("models: {}").unwrap();
        assert_eq!(
            get::<OtelConfig>(&cfg, "otel").exporter,
            OtelExporterKind::None
        );
        assert!(!get::<OtelConfig>(&cfg, "otel").wants_otlp_http());
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
  service_name: XYlitol
"#,
        )
        .expect("otel section");
        assert_eq!(
            get::<OtelConfig>(&cfg, "otel").exporter,
            OtelExporterKind::OtlpHttp
        );
        assert_eq!(
            get::<OtelConfig>(&cfg, "otel").endpoint.as_deref(),
            Some("http://127.0.0.1:3000/api/public/otel")
        );
        assert_eq!(
            get::<OtelConfig>(&cfg, "otel").protocol,
            OtelHttpProtocol::HttpBinary
        );
        assert_eq!(
            get::<OtelConfig>(&cfg, "otel").environment.as_deref(),
            Some("dev")
        );
        assert_eq!(
            get::<OtelConfig>(&cfg, "otel").service_name.as_deref(),
            Some("XYlitol")
        );
        assert_eq!(get::<OtelConfig>(&cfg, "otel").export_timeout_secs, 60);
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
        assert_eq!(get::<OtelConfig>(&cfg, "otel").export_timeout_secs, 90);
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
        assert_eq!(
            get::<OtelConfig>(&cfg, "otel").observation_io,
            OtelObservationIo::Truncated
        );
        assert_eq!(
            get::<OtelConfig>(&cfg, "otel").tool_observation_io,
            OtelObservationIo::Full
        );
    }

    #[test]
    fn otel_config_rejects_unknown_exporter() {
        let value: serde_json::Value = yaml_serde::from_str(
            r#"
models: {}
otel:
  exporter: grpc
"#,
        )
        .expect("yaml shape");
        let err = crate::validate::validate_merged_config(&value).unwrap_err();
        assert!(err.to_string().contains("otel"), "unexpected: {err}");
    }

    #[test]
    fn tui_editor_history_seed_sessions_defaults_to_one() {
        let cfg: AppConfig = yaml_serde::from_str("models: {}").expect("minimal");
        assert_eq!(
            get::<TuiConfig>(&cfg, "tui").editor_history_seed_sessions,
            1
        );
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
        assert_eq!(
            get::<TuiConfig>(&cfg, "tui").editor_history_seed_sessions,
            3
        );
    }

    #[test]
    fn tui_activity_fold_defaults() {
        let cfg: AppConfig = yaml_serde::from_str("models: {}").expect("minimal");
        assert!(get::<TuiConfig>(&cfg, "tui").activity_fold.enabled);
        assert_eq!(
            get::<TuiConfig>(&cfg, "tui")
                .activity_fold
                .keep_recent_turns,
            2
        );
        assert_eq!(
            get::<TuiConfig>(&cfg, "tui").activity_fold.stream_collapse,
            ActivityFoldStreamCollapse::Envelope
        );
        assert!(get::<TuiConfig>(&cfg, "tui").activity_fold.auto_on_rebuild);
        assert!(get::<TuiConfig>(&cfg, "tui").activity_fold.auto_on_turn_end);
        assert_eq!(
            get::<TuiConfig>(&cfg, "tui").activity_fold.display_level,
            ActivityDisplayLevel::Summary
        );
        assert!(
            get::<TuiConfig>(&cfg, "tui")
                .activity_fold
                .visible_items
                .is_none()
        );
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
    display_level: full
    visible_items:
      thinking: false
      tool_header: true
      tool_output: true
      diff: false
"#,
        )
        .expect("activity_fold");
        assert!(!get::<TuiConfig>(&cfg, "tui").activity_fold.enabled);
        assert_eq!(
            get::<TuiConfig>(&cfg, "tui")
                .activity_fold
                .keep_recent_turns,
            4
        );
        assert_eq!(
            get::<TuiConfig>(&cfg, "tui").activity_fold.stream_collapse,
            ActivityFoldStreamCollapse::Clusters
        );
        assert!(!get::<TuiConfig>(&cfg, "tui").activity_fold.auto_on_rebuild);
        assert!(!get::<TuiConfig>(&cfg, "tui").activity_fold.auto_on_turn_end);
        assert_eq!(
            get::<TuiConfig>(&cfg, "tui").activity_fold.display_level,
            ActivityDisplayLevel::Full
        );
        let visible = get::<TuiConfig>(&cfg, "tui")
            .activity_fold
            .visible_items
            .expect("visible_items");
        assert!(!visible.thinking);
        assert!(visible.tool_header);
        assert!(visible.tool_output);
        assert!(!visible.diff);
    }

    #[test]
    fn tui_activity_fold_invalid_stream_collapse_fails() {
        let value: serde_json::Value = yaml_serde::from_str(
            r#"
models: {}
tui:
  activity_fold:
    stream_collapse: envelope_please
"#,
        )
        .expect("yaml shape");
        let err = crate::validate::validate_merged_config(&value).unwrap_err();
        assert!(err.to_string().contains("tui"), "unexpected: {err}");
    }

    #[test]
    fn tool_batch_default_barrier_parallel() {
        let cfg: AppConfig = yaml_serde::from_str("models: {}").expect("minimal");
        assert_eq!(
            get::<ToolBatchConfig>(&cfg, "tool_batch").mode,
            ToolBatchMode::BarrierParallel
        );
        assert_eq!(
            xylitol_plugin_sdk::XyBatchMode::from(get::<ToolBatchConfig>(&cfg, "tool_batch").mode),
            xylitol_plugin_sdk::XyBatchMode::BarrierParallel
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
        assert_eq!(
            get::<ToolBatchConfig>(&cfg, "tool_batch").mode,
            ToolBatchMode::Sequential
        );
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
        assert_eq!(
            get::<ToolBatchConfig>(&cfg, "tool_batch").mode,
            ToolBatchMode::BarrierParallel
        );
    }

    #[test]
    fn tool_batch_invalid_mode_fails() {
        let value: serde_json::Value = yaml_serde::from_str(
            r#"
models: {}
tool_batch:
  mode: parallel
"#,
        )
        .expect("yaml shape");
        let err = crate::validate::validate_merged_config(&value).unwrap_err();
        assert!(err.to_string().contains("tool_batch"), "unexpected: {err}");
    }

    #[test]
    fn tool_batch_rejects_patterns_field() {
        let value: serde_json::Value = yaml_serde::from_str(
            r#"
models: {}
tool_batch:
  mode: sequential
  parallel_safe_patterns: ["mcp:*"]
"#,
        )
        .expect("yaml shape");
        let err = crate::validate::validate_merged_config(&value).unwrap_err();
        assert!(
            err.to_string().contains("tool_batch"),
            "MCP allowlist / patterns fields must not be accepted: {err}"
        );
    }
}

#[cfg(test)]
mod resolve_profile_model_id_tests {
    use super::*;

    #[test]
    fn resolved_profile_carries_alias_not_raw_model_name() {
        let cfg: AppConfig = yaml_serde::from_str(
            r#"
models:
  default_model: ds
  entries:
    ds:
      provider: openai
      model: deepseek-v4-flash
"#,
        )
        .expect("valid minimal config");
        let profile = cfg.resolve_default_profile().expect("default profile");
        assert_eq!(profile.model_id, "ds", "alias, not raw vendor id");
        assert_eq!(
            cfg.resolve_model("ds").expect("alias resolves").model,
            "deepseek-v4-flash",
        );
        assert_ne!(
            profile.model_id,
            cfg.resolve_model("ds").expect("alias resolves").model
        );
    }

    #[test]
    fn resolve_profile_combines_persona_before_prompt_and_carries_tools() {
        let cfg: AppConfig = yaml_serde::from_str(
            r#"
models:
  default_model: ds
  entries:
    ds:
      provider: openai
      model: deepseek-v4-flash
agents:
  default_profile: review
  profiles:
    review:
      model: ds
      persona: "You are a reviewer."
      system_prompt: "Focus on correctness."
      tools: [read, -bash]
"#,
        )
        .expect("valid profile config");
        let profile = cfg.resolve_default_profile().expect("review profile");
        assert_eq!(
            profile.system_prompt.as_deref(),
            Some("You are a reviewer.\n\nFocus on correctness.")
        );
        assert_eq!(profile.tools, vec!["read".to_string(), "-bash".to_string()]);
    }
}

#[cfg(test)]
mod external_plugins_tests {
    use super::test_support::*;
    use super::*;

    #[test]
    fn external_plugins_deserialize() {
        let cfg: AppConfig = yaml_serde::from_str(
            r#"
external_plugins:
  - name: "bun-demo"
    command: "bun"
    args: ["plugin.ts"]
    trusted: true
    timeout_ms: 5000
"#,
        )
        .expect("external plugin config");
        assert_eq!(
            get::<Vec<ExternalPluginConfig>>(&cfg, "external_plugins").len(),
            1
        );
        let plugin = &get::<Vec<ExternalPluginConfig>>(&cfg, "external_plugins")[0];
        assert_eq!(plugin.name, "bun-demo");
        assert_eq!(plugin.command, "bun");
        assert_eq!(plugin.args, vec!["plugin.ts".to_string()]);
        assert!(plugin.trusted);
        assert_eq!(plugin.timeout_ms, 5_000);
    }

    #[test]
    fn lsp_and_dap_configs_deserialize() {
        let cfg: AppConfig = yaml_serde::from_str(
            r#"
lsp:
  backend: "basedpyright-langserver --stdio"
  language: "python"
dap:
  backend: "python3 -m debugpy.adapter"
  cwd: "/tmp/project"
"#,
        )
        .expect("lsp/dap config");
        let lsp = get::<Option<LspPluginConfig>>(&cfg, "lsp").expect("lsp");
        assert_eq!(lsp.backend, "basedpyright-langserver --stdio");
        assert_eq!(lsp.language, "python");
        assert!(lsp.compression);
        let dap = get::<Option<DapPluginConfig>>(&cfg, "dap").expect("dap");
        assert_eq!(dap.backend, "python3 -m debugpy.adapter");
        assert_eq!(dap.cwd.as_deref(), Some("/tmp/project"));
        assert!(dap.compression);
    }

    #[test]
    fn subagent_config_deserializes_and_defaults_tools() {
        let cfg: AppConfig = yaml_serde::from_str(
            r#"
subagent:
  tools: [read, ls]
"#,
        )
        .expect("subagent config");
        let subagent = get::<Option<SubagentConfig>>(&cfg, "subagent").expect("subagent");
        assert_eq!(subagent.tools, vec!["read".to_string(), "ls".to_string()]);

        let cfg: AppConfig = yaml_serde::from_str(
            "subagent: {}
",
        )
        .expect("empty subagent");
        let subagent = get::<Option<SubagentConfig>>(&cfg, "subagent").expect("subagent");
        assert!(subagent.tools.contains(&"read".to_string()));
        assert!(subagent.tools.contains(&"ffgrep".to_string()));
    }

    #[test]
    fn lsp_and_dap_absent_are_none() {
        let cfg: AppConfig = yaml_serde::from_str(
            "models: {}
",
        )
        .expect("minimal config");
        assert!(get::<Option<LspPluginConfig>>(&cfg, "lsp").is_none());
        assert!(get::<Option<DapPluginConfig>>(&cfg, "dap").is_none());
    }

    #[test]
    fn external_plugin_timeout_defaults() {
        let cfg: AppConfig = yaml_serde::from_str(
            r#"
external_plugins:
  - name: "demo"
    command: "demo"
"#,
        )
        .expect("external plugin config");
        assert_eq!(
            get::<Vec<ExternalPluginConfig>>(&cfg, "external_plugins")[0].timeout_ms,
            30_000
        );
        assert!(!get::<Vec<ExternalPluginConfig>>(&cfg, "external_plugins")[0].trusted);
    }
}

#[cfg(test)]
mod tools_selection_tests {
    use super::test_support::*;
    use super::*;

    #[test]
    fn accepts_ordered_modifiers_and_bare_names() {
        let cfg: AppConfig = yaml_serde::from_str(
            r#"
tools: [read, -grep, +ffgrep]
"#,
        )
        .expect("valid tools list");
        cfg.validate_tools_selection().expect("valid");
        assert_eq!(
            get::<Vec<String>>(&cfg, "tools"),
            vec!["read", "-grep", "+ffgrep"]
        );
    }

    #[test]
    fn rejects_empty_and_prefix_only_entries() {
        for entry in ["", "   ", "+", "-"] {
            let mut cfg = AppConfig::default();
            set(&mut cfg, "tools", vec![entry.to_string()]);
            assert!(
                cfg.validate_tools_selection().is_err(),
                "must reject {entry:?}"
            );
        }
    }

    #[test]
    fn rejects_whitespace_inside_tool_name() {
        let mut cfg = AppConfig::default();
        set(&mut cfg, "tools", vec!["read write".to_string()]);
        assert!(cfg.validate_tools_selection().is_err());
    }

    #[test]
    fn unknown_tool_names_are_not_load_errors() {
        let mut cfg = AppConfig::default();
        set(&mut cfg, "tools", vec!["+no-such-tool".to_string()]);
        cfg.validate_tools_selection()
            .expect("unknown names warn later");
    }
}

#[cfg(test)]
mod serve_config_tests {
    use super::test_support::*;
    use super::*;

    #[test]
    fn omitted_serve_block_is_last_client_with_30s() {
        let cfg: AppConfig = yaml_serde::from_str("models: {}\n").expect("parse");
        assert!(get::<ServeConfig>(&cfg, "serve").autostart_last_client());
        assert_eq!(
            get::<ServeConfig>(&cfg, "serve").idle_duration(),
            std::time::Duration::from_secs(30)
        );
    }

    #[test]
    fn idle_timeout_false_forces_persistent_autostart() {
        let cfg: AppConfig = yaml_serde::from_str(
            r#"
serve:
  lifecycle: last-client
  idle_timeout: false
"#,
        )
        .expect("parse");
        assert!(!get::<ServeConfig>(&cfg, "serve").autostart_last_client());
        assert_eq!(
            get::<ServeConfig>(&cfg, "serve").lifecycle,
            ServeLifecycleConfig::LastClient
        );
    }

    #[test]
    fn trust_config_defaults_and_parse() {
        let cfg: AppConfig = yaml_serde::from_str("models: {}\n").expect("parse");
        let trust = get::<TrustConfig>(&cfg, "trust");
        assert!(trust.audit);
        assert_eq!(trust.audit_max_bytes, 10 * 1024 * 1024);
        assert_eq!(trust.audit_keep_files, 5);
        assert_eq!(trust.audit_retention_days, 90);

        let cfg: AppConfig = yaml_serde::from_str(
            r#"
models: {}
trust:
  audit: false
  audit_max_bytes: 2048
  audit_keep_files: 2
  audit_retention_days: 7
"#,
        )
        .expect("trust section");
        let trust = get::<TrustConfig>(&cfg, "trust");
        assert!(!trust.audit);
        assert_eq!(trust.audit_max_bytes, 2048);
        assert_eq!(trust.audit_keep_files, 2);
        assert_eq!(trust.audit_retention_days, 7);
        cfg.validate_trust_audit().expect("valid trust config");
    }

    #[test]
    fn trust_config_rejects_zero_keep_files() {
        let cfg: AppConfig = yaml_serde::from_str(
            r#"
models: {}
trust:
  audit_keep_files: 0
"#,
        )
        .expect("parse");
        assert!(cfg.validate_trust_audit().is_err());
    }

    #[test]
    fn explicit_persistent_lifecycle() {
        let cfg: AppConfig = yaml_serde::from_str(
            r#"
serve:
  lifecycle: persistent
  idle_timeout: 5s
"#,
        )
        .expect("parse");
        assert!(!get::<ServeConfig>(&cfg, "serve").autostart_last_client());
        assert_eq!(
            get::<ServeConfig>(&cfg, "serve").idle_duration(),
            std::time::Duration::from_secs(5)
        );
    }
}
