//! Model metadata and context-token estimates.

use crate::protocol::model::config::XyModelConfig;
use crate::protocol::model::thinking::ThinkingLevelMap;

/// LLM accounting DTOs stay single-sourced in the bridge package.
pub use xylitol_ai_bridge::dto::{ContextTokenEstimate, TokenProvenance};

/// Metadata describing a model variant available from a provider.
#[derive(Debug, Clone)]
pub struct XyModelMeta {
    pub id: String,
    pub config: XyModelConfig,
    pub display_name: String,
    pub thinking: bool,
    pub context_window: u64,
    pub api: String,
    pub provider: String,
    pub cost_input: f64,
    pub cost_output: f64,
    pub cost_cache_read: f64,
    pub cost_cache_write: f64,
    pub max_tokens: u64,
    pub thinking_levels: Vec<String>,
    /// Optional per-level provider effort/budget overrides (`null` = omit).
    pub thinking_level_map: ThinkingLevelMap,
}
