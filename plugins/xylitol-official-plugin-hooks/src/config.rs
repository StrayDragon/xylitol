//! Hook configuration vocabulary (loaded by composition-root YAML).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
    /// Optional timeout in seconds. Omit / null = dispatcher default (30s).
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
