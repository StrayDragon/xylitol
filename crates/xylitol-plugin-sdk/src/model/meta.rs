use crate::model::config::XyModelConfig;
use crate::model::thinking::ThinkingLevelMap;

pub use crate::dto::{ContextTokenEstimate, TokenProvenance};

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
    pub thinking_level_map: ThinkingLevelMap,
}
