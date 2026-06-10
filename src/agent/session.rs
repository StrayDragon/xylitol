//! AgentSession — core agent lifecycle management.
//!
//! Aligns with pi's AgentSession class. Handles:
//! - Model registry and current model tracking
//! - Thinking level toggle (low/medium/high, clamped to model)
//! - Tool registry management
//! - Session persistence integration
//! - Compaction integration
//! - Model switching (cycleForward/cycleBackward/select)
//! - Context token estimation

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::agent::model::ModelConfig;
use crate::agent::prompt::{self, SystemPromptOpts};
use crate::agent::queue::MessageQueue;
use crate::agent::tools::ToolRegistry;
use crate::agent::traits::XyModel;
use crate::agent::types::{XyContent, XyPart};
use crate::infra::session::compaction::{CompactionSettings, compact_session};
use crate::infra::session::manager::SessionManager;

// ── Thinking Level ──────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum ThinkingLevel {
    Off,
    Minimal,
    Low,
    #[default]
    Medium,
    High,
}

impl ThinkingLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            ThinkingLevel::Off => "off",
            ThinkingLevel::Minimal => "minimal",
            ThinkingLevel::Low => "low",
            ThinkingLevel::Medium => "medium",
            ThinkingLevel::High => "high",
        }
    }

    /// Clamp to what the model supports.
    pub fn clamp(self, model_supports_thinking: bool) -> Self {
        if !model_supports_thinking {
            return ThinkingLevel::Off;
        }
        self
    }
}

// ── Model Registry ──────────────────────────────────────────────────

/// Registry of available models with their configs.
#[derive(Clone, Debug, Default)]
pub struct ModelRegistry {
    models: Vec<ModelMeta>,
}

#[derive(Debug, Clone)]
pub struct ModelMeta {
    pub id: String,
    pub config: ModelConfig,
    pub display_name: String,
    pub thinking: bool,
    pub context_window: u64,
}

impl ModelRegistry {
    pub fn new() -> Self {
        Self { models: vec![] }
    }

    pub fn register(&mut self, meta: ModelMeta) {
        self.models.push(meta);
    }

    pub fn find(&self, id: &str) -> Option<&ModelMeta> {
        self.models.iter().find(|m| m.id == id)
    }

    pub fn list(&self) -> &[ModelMeta] {
        &self.models
    }

    pub fn len(&self) -> usize {
        self.models.len()
    }

    pub fn is_empty(&self) -> bool {
        self.models.is_empty()
    }

    pub fn default_model_id(&self) -> Option<&str> {
        self.models.first().map(|m| m.id.as_str())
    }
}

// ── AgentSession ────────────────────────────────────────────────────

/// Core agent session — encapsulates model, tools, session persistence, and events.
pub struct AgentSession {
    /// Model registry for model switching.
    model_registry: ModelRegistry,
    /// Current model index in the registry.
    current_model_index: usize,
    /// Tool registry with builtin tools.
    tool_registry: ToolRegistry,
    /// Session persistence manager.
    session_manager: SessionManager,
    /// Current thinking level.
    thinking_level: ThinkingLevel,
    /// System prompt to prepend to every turn.
    system_prompt: Option<String>,
    /// Current session ID.
    session_id: Option<String>,
    /// Max ReAct loop iterations per turn.
    max_iterations: u32,
    /// Context window threshold for compaction (0.0-1.0).
    compaction_threshold: f64,
    /// CWD for session header.
    cwd: String,
    /// System prompt options for dynamic building.
    prompt_opts: SystemPromptOpts,
    /// Message queue for steer/followUp.
    message_queue: MessageQueue,
}

impl AgentSession {
    pub fn new(
        model_registry: ModelRegistry,
        tool_registry: ToolRegistry,
        session_manager: SessionManager,
        system_prompt: Option<String>,
        max_iterations: u32,
        compaction_threshold: f64,
        cwd: String,
    ) -> Self {
        Self {
            model_registry,
            current_model_index: 0,
            tool_registry,
            session_manager,
            thinking_level: ThinkingLevel::default(),
            system_prompt,
            session_id: None,
            max_iterations,
            compaction_threshold,
            cwd: cwd.clone(),
            prompt_opts: SystemPromptOpts {
                cwd,
                ..Default::default()
            },
            message_queue: MessageQueue::new(),
        }
    }

    // ── Model management ──────────────────────────────────────────

    /// Get the current model config.
    pub fn current_model(&self) -> Option<&ModelMeta> {
        self.model_registry.models.get(self.current_model_index)
    }

    /// Build the current model instance.
    pub fn build_current_model(&self) -> Result<Arc<dyn XyModel>, String> {
        let meta = self
            .current_model()
            .ok_or_else(|| "no model configured".to_string())?;
        meta.config.build()
    }

    /// Get current thinking level (clamped).
    pub fn thinking_level(&self) -> ThinkingLevel {
        let supports = self.current_model().map(|m| m.thinking).unwrap_or(false);
        self.thinking_level.clamp(supports)
    }

    /// Set thinking level.
    pub fn set_thinking_level(&mut self, level: ThinkingLevel) {
        self.thinking_level = level;
        // Fire-and-forget persistence (async call from sync context OK in tokio tests)
        if let Some(ref sid) = self.session_id {
            let mgr = self.session_manager.clone();
            let sid = sid.clone();
            let level_str = level.as_str().to_string();
            tokio::spawn(async move {
                let _ = mgr.append_thinking_level_change(&sid, &level_str).await;
            });
        }
    }

    /// Cycle to the next model.
    pub fn cycle_forward(&mut self) -> Option<&ModelMeta> {
        if self.model_registry.is_empty() {
            return None;
        }
        self.current_model_index = (self.current_model_index + 1) % self.model_registry.len();
        self.current_model()
    }

    /// Cycle to the previous model.
    pub fn cycle_backward(&mut self) -> Option<&ModelMeta> {
        if self.model_registry.is_empty() {
            return None;
        }
        self.current_model_index = if self.current_model_index == 0 {
            self.model_registry.len() - 1
        } else {
            self.current_model_index - 1
        };
        self.current_model()
    }

    /// Select a specific model by ID.
    pub fn select_model(&mut self, model_id: &str) -> Result<(), String> {
        let idx = self
            .model_registry
            .models
            .iter()
            .position(|m| m.id == model_id)
            .ok_or_else(|| format!("model not found: {model_id}"))?;
        self.current_model_index = idx;
        // Fire-and-forget persistence
        if let Some(ref sid) = self.session_id {
            let mgr = self.session_manager.clone();
            let sid = sid.clone();
            let mid = model_id.to_string();
            tokio::spawn(async move {
                let _ = mgr.append_model_change(&sid, &mid, &mid).await;
            });
        }
        Ok(())
    }

    // ── Session management ────────────────────────────────────────

    /// Set the active session ID.
    pub fn set_session(&mut self, session_id: String) {
        self.session_id = Some(session_id);
    }

    /// Get the active session ID.
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// Ensure a session exists (create if needed).
    pub async fn ensure_session(&self, id: &str, parent: Option<&str>) -> Result<(), String> {
        if !self.session_manager.exists(id) {
            let cwd_clone = self.cwd.clone();
            self.session_manager
                .create(id, Some(&cwd_clone), parent)
                .await?;
        }
        Ok(())
    }

    // ── Accessors ─────────────────────────────────────────────────

    pub fn tool_registry(&self) -> &ToolRegistry {
        &self.tool_registry
    }

    pub fn session_manager(&self) -> &SessionManager {
        &self.session_manager
    }

    pub fn system_prompt(&self) -> Option<&str> {
        self.system_prompt.as_deref()
    }

    pub fn max_iterations(&self) -> u32 {
        self.max_iterations
    }

    pub fn compaction_threshold(&self) -> f64 {
        self.compaction_threshold
    }

    pub fn model_registry(&self) -> &ModelRegistry {
        &self.model_registry
    }

    // ── Compaction ───────────────────────────────────────────────

    /// Compact the current session, summarizing old entries via LLM.
    ///
    /// Requires an active session and a configured model. Writes a
    /// CompactionEntry to the session file.
    pub async fn compact_current_session(&self, model: &dyn XyModel) -> Result<(), String> {
        let sid = self
            .session_id()
            .ok_or_else(|| "no active session".to_string())?;

        let settings = CompactionSettings {
            enabled: true,
            reserve_tokens: 16384,
            keep_recent_tokens: 20000,
        };

        compact_session(&self.session_manager, sid, model, &settings)
            .await
            .map_err(|e| format!("compaction failed: {e}"))?;

        Ok(())
    }

    // ── Fork ────────────────────────────────────────────────────

    /// Fork the current session at a given entry, creating a child session.
    ///
    /// Returns the child session ID on success.
    pub async fn fork_session(&self, at_entry_id: &str) -> Result<String, String> {
        let parent_id = self
            .session_id()
            .ok_or_else(|| "no active session".to_string())?;

        let child_id = uuid::Uuid::new_v4().to_string();

        self.session_manager
            .fork(parent_id, &child_id, at_entry_id)
            .await
            .map_err(|e| format!("fork failed: {e}"))?;

        Ok(child_id)
    }

    // ── Dynamic system prompt ────────────────────────────────────

    /// Set active tools by name and rebuild the system prompt.
    pub fn set_active_tools(&mut self, tool_names: &[String]) {
        self.prompt_opts.selected_tools = tool_names.to_vec();
        self.prompt_opts.tool_snippets =
            prompt::collect_tool_snippets(&self.tool_registry, tool_names);
        self.rebuild_system_prompt();
    }

    /// Rebuild the system prompt from current options.
    pub fn rebuild_system_prompt(&mut self) {
        self.system_prompt = Some(prompt::build_system_prompt(&self.prompt_opts));
    }

    /// Set append system prompt text.
    pub fn set_append_prompt(&mut self, text: Option<String>) {
        self.prompt_opts.append_prompt = text;
        self.rebuild_system_prompt();
    }

    // ── Message queue ────────────────────────────────────────────

    pub fn message_queue(&self) -> &MessageQueue {
        &self.message_queue
    }

    pub fn message_queue_mut(&mut self) -> &mut MessageQueue {
        &mut self.message_queue
    }

    // ── Session stats ────────────────────────────────────────────

    /// Get session statistics.
    pub async fn get_session_stats(&self) -> Result<SessionStats, String> {
        let sid = self
            .session_id()
            .ok_or_else(|| "no active session".to_string())?;
        let ctx = self.session_manager.build_session_context(sid).await?;

        let user_messages = ctx
            .messages
            .iter()
            .filter(|m| m.get("role").and_then(|r| r.as_str()) == Some("user"))
            .count();
        let assistant_messages = ctx
            .messages
            .iter()
            .filter(|m| m.get("role").and_then(|r| r.as_str()) == Some("assistant"))
            .count();
        let total_messages = ctx.messages.len();

        Ok(SessionStats {
            session_id: sid.to_string(),
            user_messages,
            assistant_messages,
            total_messages,
            thinking_level: ctx.thinking_level,
            model: ctx.model,
        })
    }

    /// Send a custom message to the session.
    pub async fn send_custom_message(
        &self,
        custom_type: &str,
        content: serde_json::Value,
        display: bool,
    ) -> Result<(), String> {
        let sid = self
            .session_id()
            .ok_or_else(|| "no active session".to_string())?;
        self.session_manager
            .append_custom_message(sid, custom_type, content, display, None)
            .await
    }

    /// Check and perform auto-compaction if the context is full.
    /// Returns true if compaction was performed.
    pub async fn maybe_auto_compact(&self) -> Result<bool, String> {
        let sid = self
            .session_id()
            .ok_or_else(|| "no active session".to_string())?;

        let model = self
            .build_current_model()
            .map_err(|e| format!("no model: {e}"))?;

        let ctx_window = self
            .current_model()
            .map(|m| m.context_window)
            .unwrap_or(128000);

        let session_ctx = self.session_manager.build_session_context(sid).await?;
        let token_estimate: u64 = session_ctx
            .messages
            .iter()
            .map(|m| (m.to_string().len() as u64).div_ceil(4))
            .sum();

        if !should_compact(token_estimate, ctx_window, self.compaction_threshold) {
            return Ok(false);
        }

        let settings = CompactionSettings {
            enabled: true,
            reserve_tokens: 16384,
            keep_recent_tokens: 20000,
        };

        compact_session(&self.session_manager, sid, model.as_ref(), &settings)
            .await
            .map_err(|e| format!("auto-compaction: {e}"))?;

        Ok(true)
    }
}

#[derive(Debug, Clone)]
pub struct SessionStats {
    pub session_id: String,
    pub user_messages: usize,
    pub assistant_messages: usize,
    pub total_messages: usize,
    pub thinking_level: String,
    pub model: Option<(String, String)>,
}

// ── Context estimation ──────────────────────────────────────────────

/// Estimate token count from messages using simple heuristic (1 token ≈ 4 chars).
pub fn estimate_tokens(messages: &[XyContent]) -> u64 {
    let mut total = 0u64;
    for msg in messages {
        for part in &msg.parts {
            match part {
                XyPart::Text(s) | XyPart::Thinking(s) => {
                    total += (s.len() as u64).div_ceil(4);
                }
                XyPart::FunctionCall { name, args, id: _ } => {
                    total += (name.len() as u64).div_ceil(4);
                    total += (args.to_string().len() as u64).div_ceil(4);
                }
                XyPart::FunctionResponse {
                    name: _,
                    result,
                    id: _,
                } => {
                    total += (result.len() as u64).div_ceil(4);
                }
            }
        }
    }
    total
}

/// Check if compaction should be triggered.
pub fn should_compact(token_estimate: u64, context_window: u64, threshold: f64) -> bool {
    if context_window == 0 {
        return false;
    }
    let usage_ratio = token_estimate as f64 / context_window as f64;
    usage_ratio >= threshold
}

#[derive(Debug, Clone)]
pub struct ContextUsage {
    pub tokens: u64,
    pub context_window: u64,
    pub percent: u64,
    pub should_compact: bool,
}

/// Get context usage info for the current session state.
pub fn get_context_usage(token_estimate: u64, context_window: u64, threshold: f64) -> ContextUsage {
    let percent = if context_window > 0 {
        ((token_estimate as f64 / context_window as f64) * 100.0) as u64
    } else {
        0
    };
    ContextUsage {
        tokens: token_estimate,
        context_window,
        percent,
        should_compact: should_compact(token_estimate, context_window, threshold),
    }
}
