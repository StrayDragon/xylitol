//! Generate-call options carried across the [`crate::ports::XyModel`] boundary.

use std::collections::HashMap;

use fastrace::prelude::SpanContext;

use crate::model::{THINKING_OFF, ThinkingBudgets};

/// Snapshot of xylitol bookmark identity for one generate / turn (c2590).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObsSessionContext {
    /// Xylitol session id (JSONL / user-switchable). Always a product fact when known.
    pub session_id: Option<String>,
    pub session_name: Option<String>,
    /// Parent xylitol session (header `parentSession`). Omit when this is not a fork.
    pub parent_session_id: Option<String>,
    /// Fork cut entry id (header `forkAtEntryId`). Omit when unknown / not a fork.
    pub fork_at_entry_id: Option<String>,
    /// Session identity actually presented on the LLM channel for this request.
    /// Omit when the request did not send one (do not fill with `session_id` as a placeholder).
    pub llm_gateway_session_id: Option<String>,
    /// Compaction summary: configured task model wire id (c2811).
    pub compaction_requested_model: Option<String>,
    /// Compaction summary: actual model wire id used for the request.
    pub compaction_actual_model: Option<String>,
    /// Compaction summary: true when task entry build failed and current model was used.
    pub compaction_model_fallback: Option<bool>,
    /// Compaction summary: true when thinking_level override was outside the support set.
    pub compaction_thinking_rejected: Option<bool>,
}

/// Options for a single [`crate::ports::XyModel::generate_stream`] call.
#[derive(Debug, Clone)]
pub struct XyGenerateOptions {
    /// Freeform configured level string; known names have Anthropic budget fallbacks.
    pub thinking_level: String,
    pub level_map: HashMap<String, Option<String>>,
    pub thinking_budgets: Option<ThinkingBudgets>,
    /// System prompt injected each request via the adapter's formal channel
    /// (Responses `developer`/`system`, Completions system message, Anthropic `system`).
    pub system_prompt: Option<String>,
    /// Output budget for this single request (Anthropic `max_tokens`, Completions
    /// `max_completion_tokens`, Responses `max_output_tokens`). `None` = adapter
    /// default (historical behavior).
    pub max_output_tokens: Option<u32>,
    /// Optional fastrace parent for `llm.request` nesting (iteration / compaction).
    pub obs_parent: Option<SpanContext>,
    /// Generate-scoped obs session snapshot. HTTP attribution and `llm.request`
    /// MUST use this copy; overlapping generate MUST NOT re-read the process slot.
    /// Empty (`Default`) means no session on this call (idle paths may still
    /// bind a Client without a snapshot and fall back to the process slot).
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
            obs_parent: None,
            obs_session: ObsSessionContext::default(),
        }
    }
}
