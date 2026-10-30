use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::error::LoadError;

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AppConfig {
    #[serde(rename = "models")]
    #[schemars(rename = "models")]
    pub model: ModelsConfig,
    pub agents: AgentsConfig,
    pub execution: ExecutionConfig,
    pub hooks: HooksConfig,
    pub security: SecurityConfig,
    pub session: Option<SessionConfig>,
    pub compaction: Option<xylitol_plugin_sdk::compaction_config::XyCompactionSettingsConfig>,

    pub mcp_servers: Option<Vec<McpServerConfig>>,

    #[serde(default)]
    pub otel: OtelConfig,

    #[serde(default)]
    pub tui: TuiConfig,

    #[serde(default)]
    pub tool_batch: ToolBatchConfig,

    #[serde(default)]
    pub tools: Vec<String>,

    #[serde(default)]
    pub loop_policies: LoopPoliciesConfig,

    #[serde(default)]
    pub todo: TodoConfig,

    #[serde(default)]
    pub workflow: WorkflowConfig,

    #[serde(default)]
    pub plan: PlanConfig,

    #[serde(default)]
    pub external_plugins: Vec<ExternalPluginConfig>,

    #[serde(default)]
    pub lsp: Option<LspPluginConfig>,

    #[serde(default)]
    pub dap: Option<DapPluginConfig>,

    #[serde(default)]
    pub subagent: Option<SubagentConfig>,
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActivityFoldStreamCollapse {
    #[default]
    Envelope,
    Clusters,
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
#[serde(default)]
pub struct ModelsConfig {
    pub default_model: Option<String>,
    pub models: HashMap<String, ModelEntry>,
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
    pub fn validate_thinking_levels(&self) -> Result<(), LoadError> {
        for (alias, entry) in &self.model.models {
            entry
                .resolve_thinking_config()
                .map_err(|e| LoadError::validation(format!("models.{alias}: {e}")))?;
        }
        Ok(())
    }

    pub fn validate_session_max_turns(&self) -> Result<(), LoadError> {
        match self.session.as_ref().and_then(|s| s.max_turns) {
            None => Ok(()),
            Some(0) => Err(LoadError::validation(
                "session.max_turns must be a positive integer (got 0)",
            )),
            Some(_) => Ok(()),
        }
    }

    pub fn validate_tools_selection(&self) -> Result<(), LoadError> {
        for (index, raw) in self.tools.iter().enumerate() {
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
        let entry = self.model.models.get(model_id).ok_or_else(|| {
            LoadError::validation(format!(
                "unknown model alias `{model_id}`: add it under models.models in config.yaml"
            ))
        })?;
        Ok(entry.to_model_config())
    }

    pub fn resolve_model_meta(
        &self,
        model_id: &str,
    ) -> Result<xylitol_plugin_sdk::model::XyModelMeta, LoadError> {
        let entry = self.model.models.get(model_id).ok_or_else(|| {
            LoadError::validation(format!(
                "unknown model alias `{model_id}`: add it under models.models in config.yaml"
            ))
        })?;
        entry.to_model_meta(model_id).map_err(LoadError::validation)
    }

    pub fn resolve_profile(
        &self,
        name: &str,
    ) -> Result<xylitol_plugin_sdk::model::ResolvedProfile, LoadError> {
        let profile = self.agents.profiles.get(name);

        let (model_ref, system_prompt, tools) = match profile {
            Some(p) => (
                p.model.as_deref(),
                compose_profile_prompt(p, self.execution.system_prompt.as_ref()),
                p.tools.clone(),
            ),
            None => (None, self.execution.system_prompt.clone(), Vec::new()),
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
        let name = if self.agents.default_profile.is_empty() {
            "default"
        } else {
            &self.agents.default_profile
        };
        self.resolve_profile(name)
    }
}

pub use xylitol_official_plugin_hooks::{HookEntry, HooksConfig};
pub use xylitol_official_plugin_loop_policies::LoopPoliciesConfig;
pub use xylitol_official_plugin_plan::PlanConfig;
pub use xylitol_official_plugin_todo::TodoConfig;
pub use xylitol_official_plugin_workflow::WorkflowConfig;

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

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct SessionConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_turns: Option<u32>,
}

pub use xylitol_official_plugin_dap::DapPluginConfig;
pub use xylitol_official_plugin_lsp::LspPluginConfig;
pub use xylitol_official_plugin_mcp::{McpServerConfig, McpTransportKind};

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
mod session_max_turns_tests {
    use super::*;

    #[test]
    fn session_max_turns_deserializes() {
        let cfg: AppConfig = yaml_serde::from_str(
            r#"
session:
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
    use xylitol_plugin_sdk::model::XyModelKind;

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
            input_modalities: None,
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
            xylitol_plugin_sdk::XyBatchMode::from(cfg.tool_batch.mode),
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

#[cfg(test)]
mod resolve_profile_model_id_tests {
    use super::*;

    #[test]
    fn resolved_profile_carries_alias_not_raw_model_name() {
        let cfg: AppConfig = yaml_serde::from_str(
            r#"
models:
  default_model: ds
  models:
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
  models:
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
        assert_eq!(cfg.external_plugins.len(), 1);
        let plugin = &cfg.external_plugins[0];
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
        let lsp = cfg.lsp.expect("lsp");
        assert_eq!(lsp.backend, "basedpyright-langserver --stdio");
        assert_eq!(lsp.language, "python");
        assert!(lsp.compression);
        let dap = cfg.dap.expect("dap");
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
        let subagent = cfg.subagent.expect("subagent");
        assert_eq!(subagent.tools, vec!["read".to_string(), "ls".to_string()]);

        let cfg: AppConfig = yaml_serde::from_str(
            "subagent: {}
",
        )
        .expect("empty subagent");
        let subagent = cfg.subagent.expect("subagent");
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
        assert!(cfg.lsp.is_none());
        assert!(cfg.dap.is_none());
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
        assert_eq!(cfg.external_plugins[0].timeout_ms, 30_000);
        assert!(!cfg.external_plugins[0].trusted);
    }
}

#[cfg(test)]
mod tools_selection_tests {
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
        assert_eq!(cfg.tools, vec!["read", "-grep", "+ffgrep"]);
    }

    #[test]
    fn rejects_empty_and_prefix_only_entries() {
        for entry in ["", "   ", "+", "-"] {
            let cfg = AppConfig {
                tools: vec![entry.into()],
                ..Default::default()
            };
            assert!(
                cfg.validate_tools_selection().is_err(),
                "must reject {entry:?}"
            );
        }
    }

    #[test]
    fn rejects_whitespace_inside_tool_name() {
        let cfg = AppConfig {
            tools: vec!["read write".into()],
            ..Default::default()
        };
        assert!(cfg.validate_tools_selection().is_err());
    }

    #[test]
    fn unknown_tool_names_are_not_load_errors() {
        let cfg = AppConfig {
            tools: vec!["+no-such-tool".into()],
            ..Default::default()
        };
        cfg.validate_tools_selection()
            .expect("unknown names warn later");
    }
}
