use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use serde::Serialize;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use super::error::{XyError, XyToolError};
use super::message::AgentMessage;
use super::types::{XyChunk, XyToolSchema};

pub type XyStream = Pin<Box<dyn Stream<Item = Result<XyChunk, XyError>> + Send>>;

#[async_trait]
pub trait XyModel: Send + Sync {
    fn name(&self) -> &str;

    /// Generate a streaming response from AgentMessage history.
    /// This is the canonical provider interface.
    async fn generate_stream(
        &self,
        messages: Vec<AgentMessage>,
        tools: &[XyToolSchema],
        stream: bool,
    ) -> Result<XyStream, XyError>;
}

/// Context passed to tool execution.
#[derive(Clone)]
pub struct XyToolCtx {
    /// Unique identifier for this tool call.
    pub call_id: String,
    /// Cancellation token — tools should check this and abort if cancelled.
    pub cancel: CancellationToken,
}

impl XyToolCtx {
    pub fn new(call_id: impl Into<String>) -> Self {
        Self {
            call_id: call_id.into(),
            cancel: CancellationToken::new(),
        }
    }

    pub fn with_cancel(call_id: impl Into<String>, cancel: CancellationToken) -> Self {
        Self {
            call_id: call_id.into(),
            cancel,
        }
    }
}

/// Whether a tool prefers sequential or parallel execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize)]
pub enum ToolExecutionMode {
    /// Execute in parallel with other tools (default).
    #[default]
    Parallel,
    /// Execute sequentially (entire batch falls back to sequential).
    Sequential,
}

/// Tool trait — all tools must implement this.
///
/// The `execute` method receives a `XyToolCtx` which contains a `CancellationToken`.
/// Tools MUST:
/// 1. Check `ctx.cancel.is_cancelled()` at appropriate checkpoints
/// 2. Return `XyToolError::Aborted` when cancellation is detected
#[async_trait]
pub trait XyTool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> Value;
    async fn execute(&self, ctx: &XyToolCtx, args: Value) -> Result<String, XyToolError>;

    /// One-line prompt snippet for system prompt construction.
    /// Default returns a trimmed version of the description.
    fn prompt_snippet(&self) -> Option<&str> {
        None
    }

    /// Prompt guidelines for system prompt (e.g. usage examples).
    fn prompt_guidelines(&self) -> &[&str] {
        &[]
    }

    /// Whether this tool prefers sequential execution.
    /// When any tool in a batch declares `Sequential`, the entire batch
    /// falls back to sequential execution.
    fn execution_mode(&self) -> ToolExecutionMode {
        ToolExecutionMode::Parallel
    }

    /// Optional argument preprocessing before execution.
    /// Default: pass through unchanged (identity).
    fn prepare_arguments(&self, args: Value) -> Value {
        args
    }
}

// ── ToolDefinition ──────────────────────────────────────────────────

/// Unified tool definition — standardises prompt display for all tools.
#[derive(Debug, Clone, Serialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
    pub prompt_snippet: Option<String>,
    pub prompt_guidelines: Vec<String>,
    pub execution_mode: ToolExecutionMode,
    pub source_info: Option<crate::infra::source_info::SourceInfo>,
}

impl Default for ToolDefinition {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            parameters: Value::Null,
            prompt_snippet: None,
            prompt_guidelines: Vec::new(),
            execution_mode: ToolExecutionMode::Parallel,
            source_info: None,
        }
    }
}

impl<'a> From<&'a dyn XyTool> for ToolDefinition {
    fn from(tool: &'a dyn XyTool) -> Self {
        let prompt_snippet = tool
            .prompt_snippet()
            .map(|s| s.to_string())
            .or_else(|| {
                let desc = tool.description();
                if desc.is_empty() {
                    None
                } else {
                    // Truncate to first 80 chars as snippet
                    let snippet: String = desc.chars().take(80).collect();
                    if desc.len() > 80 {
                        Some(format!("{snippet}…"))
                    } else {
                        Some(snippet)
                    }
                }
            });

        let prompt_guidelines = tool
            .prompt_guidelines()
            .iter()
            .map(|s| s.to_string())
            .collect();

        Self {
            name: tool.name().to_string(),
            description: tool.description().to_string(),
            parameters: tool.parameters_schema(),
            prompt_snippet,
            prompt_guidelines,
            execution_mode: tool.execution_mode(),
            source_info: None,
        }
    }
}
