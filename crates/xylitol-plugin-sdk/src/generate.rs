use std::collections::HashMap;

use fastrace::prelude::SpanContext;

use crate::model::{THINKING_OFF, ThinkingBudgets};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObsSessionContext {
    pub session_id: Option<String>,
    pub session_name: Option<String>,
    pub parent_session_id: Option<String>,
    pub fork_at_entry_id: Option<String>,
    pub llm_gateway_session_id: Option<String>,
    pub compaction_requested_model: Option<String>,
    pub compaction_actual_model: Option<String>,
    pub compaction_model_fallback: Option<bool>,
    pub compaction_thinking_rejected: Option<bool>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct XyStructuredOutput {
    pub name: String,
    pub schema: serde_json::Value,
    pub strict: Option<bool>,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct XyGenerateOptions {
    pub thinking_level: String,
    pub level_map: HashMap<String, Option<String>>,
    pub thinking_budgets: Option<ThinkingBudgets>,
    pub system_prompt: Option<String>,
    pub max_output_tokens: Option<u32>,
    pub structured_output: Option<XyStructuredOutput>,
    pub obs_parent: Option<SpanContext>,
    pub obs_session: ObsSessionContext,
}

impl Default for XyGenerateOptions {
    fn default() -> Self {
        Self {
            thinking_level: THINKING_OFF.into(),
            level_map: HashMap::new(),
            thinking_budgets: None,
            system_prompt: None,
            max_output_tokens: None,
            structured_output: None,
            obs_parent: None,
            obs_session: ObsSessionContext::default(),
        }
    }
}

pub const XYLITOL_OBS_LANE_ATTR: &str = "xylitol.obs.lane";
pub const XYLITOL_OBS_LANE_LLM: &str = "llm";

pub fn xylitol_obs_lane_properties(lane: &str) -> Vec<(String, String)> {
    vec![(XYLITOL_OBS_LANE_ATTR.into(), lane.to_string())]
}

pub fn langfuse_session_properties_from(ctx: &ObsSessionContext) -> Vec<(String, String)> {
    let mut out = Vec::with_capacity(6);
    if let Some(id) = ctx.session_id.clone().filter(|s| !s.is_empty()) {
        out.push(("langfuse.session.id".into(), id.clone()));
        out.push(("xylitol.session.id".into(), id));
    }
    if let Some(gw) = ctx.llm_gateway_session_id.clone().filter(|s| !s.is_empty()) {
        out.push(("xylitol.session.llm_gateway_session_id".into(), gw));
    }
    if let Some(parent) = ctx.parent_session_id.clone().filter(|s| !s.is_empty()) {
        out.push(("xylitol.session.parent_session_id".into(), parent));
    }
    if let Some(cut) = ctx.fork_at_entry_id.clone().filter(|s| !s.is_empty()) {
        out.push(("xylitol.session.fork_at_entry_id".into(), cut));
    }
    if let Some(name) = ctx.session_name.clone().filter(|s| !s.is_empty()) {
        out.push(("langfuse.trace.metadata.session_name".into(), name));
    }
    if let Some(model) = ctx
        .compaction_requested_model
        .clone()
        .filter(|s| !s.is_empty())
    {
        out.push(("xylitol.compaction.requested_model".into(), model));
    }
    if let Some(model) = ctx
        .compaction_actual_model
        .clone()
        .filter(|s| !s.is_empty())
    {
        out.push(("xylitol.compaction.actual_model".into(), model));
    }
    if let Some(fallback) = ctx.compaction_model_fallback {
        out.push((
            "xylitol.compaction.model_fallback".into(),
            fallback.to_string(),
        ));
    }
    if let Some(rejected) = ctx.compaction_thinking_rejected {
        out.push((
            "xylitol.compaction.thinking_rejected".into(),
            rejected.to_string(),
        ));
    }
    out
}

pub fn langfuse_observation_properties_from(
    observation_type: &str,
    ctx: &ObsSessionContext,
) -> Vec<(String, String)> {
    let mut out = vec![(
        "langfuse.observation.type".into(),
        observation_type.to_string(),
    )];
    out.extend(langfuse_session_properties_from(ctx));
    out.extend(xylitol_obs_lane_properties(XYLITOL_OBS_LANE_LLM));
    out
}
