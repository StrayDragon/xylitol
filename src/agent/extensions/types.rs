//! Extension system types — aligns with pi's extensions/types.ts.
//!
//! Key types:
//! - `ExtensionContext` — cwd, session, model, signal, abort
//! - `ToolDefinition` — tool name, description, schema, execute
//! - Extension lifecycle events

use std::sync::Arc;

use serde_json::Value;

use crate::agent::registry::ModelRegistry;
use crate::infra::session::SessionManager;

// ── ExtensionContext ─────────────────────────────────────────────────

/// ExtensionContext provides cwd, session_manager, model_registry, model, signal, abort.
///
/// Aligns with pi's `ExtensionContext` interface (extension types).
#[derive(Clone)]
pub struct ExtensionContext {
    /// Current working directory.
    pub cwd: String,
    /// Session manager (read-write access).
    pub session_manager: Arc<SessionManager>,
    /// Model registry for model switching.
    pub model_registry: Arc<ModelRegistry>,
    /// Current model id (None if not set).
    pub model_id: Option<String>,
    /// Abort signal for cancellation.
    pub signal: Option<Arc<tokio_util::sync::CancellationToken>>,
}

impl ExtensionContext {
    /// Create a new extension context.
    pub fn new(
        cwd: String,
        session_manager: Arc<SessionManager>,
        model_registry: Arc<ModelRegistry>,
        model_id: Option<String>,
        signal: Option<Arc<tokio_util::sync::CancellationToken>>,
    ) -> Self {
        Self {
            cwd,
            session_manager,
            model_registry,
            model_id,
            signal,
        }
    }

    /// Abort the current agent operation.
    pub fn abort(&self) {
        if let Some(ref s) = self.signal {
            s.cancel();
        }
    }

    /// Check if the operation has been aborted.
    pub fn is_aborted(&self) -> bool {
        self.signal.as_ref().is_some_and(|s| s.is_cancelled())
    }
}

impl std::fmt::Debug for ExtensionContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExtensionContext")
            .field("cwd", &self.cwd)
            .field("model_id", &self.model_id)
            .finish()
    }
}

// ── ToolDefinition ───────────────────────────────────────────────────

/// ToolDefinition trait — aligns with pi's ToolDefinition interface.
///
/// Extensions register tools by implementing this trait.
/// Tools are LLM-callable and participate in the agent loop.
pub trait ToolDefinition: Send + Sync {
    /// Tool name used in LLM tool calls.
    fn name(&self) -> &str;

    /// Human-readable description for the LLM.
    fn description(&self) -> &str;

    /// Optional one-line snippet for the system prompt.
    fn prompt_snippet(&self) -> Option<&str> {
        None
    }

    /// JSON Schema for the tool parameters (as serde_json::Value).
    fn parameters_schema(&self) -> Value;

    /// Optional guideline bullets for the system prompt.
    fn prompt_guidelines(&self) -> Vec<String> {
        vec![]
    }

    /// Execute the tool with the given parameters.
    ///
    /// Returns a JSON value result (AgentToolResult equivalent).
    fn execute(
        &self,
        tool_call_id: &str,
        params: Value,
        signal: Option<Arc<tokio_util::sync::CancellationToken>>,
        ctx: &ExtensionContext,
    ) -> Result<Value, String>;
}

/// A registered tool entry with source information.
#[derive(Clone)]
pub struct RegisteredTool {
    pub definition: Arc<dyn ToolDefinition>,
    pub source_info: ToolSourceInfo,
}

/// Metadata about where a tool was registered from.
#[derive(Debug, Clone)]
pub struct ToolSourceInfo {
    pub extension_name: String,
    pub source_path: Option<String>,
}

// ── Tool hooks ──────────────────────────────────────────────────────

/// before_tool_call result — can block tool execution.
#[derive(Debug, Clone)]
pub struct ToolCallHookResult {
    /// Whether to block the tool execution.
    pub block: bool,
    /// Reason for blocking (shown to user).
    pub reason: Option<String>,
}

/// after_tool_call result — can modify the tool result.
#[derive(Debug, Clone)]
pub struct ToolResultHookResult {
    /// Modified tool content (optional).
    pub content: Option<Value>,
    /// Whether the result is an error.
    pub is_error: Option<bool>,
}

// ── Extension lifecycle event types ─────────────────────────────────

/// Agent lifecycle events that extensions can subscribe to.
#[derive(Debug, Clone)]
pub enum ExtensionEvent {
    /// Agent loop started.
    AgentStart,
    /// Agent loop ended (with final messages).
    AgentEnd { messages: Vec<Value> },
    /// A turn started.
    TurnStart { turn_index: usize },
    /// A turn ended.
    TurnEnd { turn_index: usize, message: Value },
    /// A tool started executing.
    ToolExecutionStart {
        tool_call_id: String,
        tool_name: String,
        args: Value,
    },
    /// A tool execution produced partial output.
    ToolExecutionUpdate {
        tool_call_id: String,
        tool_name: String,
        partial_result: Value,
    },
    /// A tool finished executing.
    ToolExecutionEnd {
        tool_call_id: String,
        tool_name: String,
        result: Value,
        is_error: bool,
    },
    /// Before an LLM call (context assembled).
    Context { messages: Vec<Value> },
    /// Model selected/changed.
    ModelSelect {
        model_id: String,
        previous_model_id: Option<String>,
    },
    /// Thinking level changed.
    ThinkingLevelSelect {
        level: String,
        previous_level: String,
    },
    /// Before tool call (can block).
    BeforeToolCall {
        tool_call_id: String,
        tool_name: String,
        args: Value,
    },
    /// After tool call (can modify result).
    AfterToolCall {
        tool_call_id: String,
        tool_name: String,
        result: Value,
        is_error: bool,
    },
}

impl ExtensionEvent {
    /// Get the event type name for channel subscription.
    pub fn event_type(&self) -> &str {
        match self {
            ExtensionEvent::AgentStart => "agent_start",
            ExtensionEvent::AgentEnd { .. } => "agent_end",
            ExtensionEvent::TurnStart { .. } => "turn_start",
            ExtensionEvent::TurnEnd { .. } => "turn_end",
            ExtensionEvent::ToolExecutionStart { .. } => "tool_execution_start",
            ExtensionEvent::ToolExecutionUpdate { .. } => "tool_execution_update",
            ExtensionEvent::ToolExecutionEnd { .. } => "tool_execution_end",
            ExtensionEvent::Context { .. } => "context",
            ExtensionEvent::ModelSelect { .. } => "model_select",
            ExtensionEvent::ThinkingLevelSelect { .. } => "thinking_level_select",
            ExtensionEvent::BeforeToolCall { .. } => "before_tool_call",
            ExtensionEvent::AfterToolCall { .. } => "after_tool_call",
        }
    }
}
