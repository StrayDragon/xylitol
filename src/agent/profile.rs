//! Resolved agent profile — runtime representation of an agent configuration.

use crate::agent::model::ModelConfig;

/// Fully resolved agent profile, ready for [`crate::agent::r#loop::AgentLoop`] construction.
///
/// Produced by [`crate::infra::config::types::AppConfig::resolve_profile`] from
/// config-level [`AgentProfile`](crate::infra::config::types::AgentProfile) entries.
#[derive(Debug, Clone)]
pub struct ResolvedProfile {
    /// Agent-level model config (has kind, api_key, model name, base_url).
    pub model_config: ModelConfig,
    /// System prompt override for this agent.
    pub system_prompt: Option<String>,
    /// Allowed tool names. `None` means all tools available.
    pub allowed_tools: Option<Vec<String>>,
    /// Maximum ReAct loop iterations for this agent.
    pub max_iterations: u32,
    /// Profile name (for logging and diagnostics).
    pub name: String,
}
