//! Core abstractions for models and tools.
//!
//! Defines [`XyModel`] (LLM provider contract) and [`XyTool`]
//! (tool execution contract), along with shared context types.

use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::core::error::{XyError, XyToolError};
use crate::core::message::AgentMessage;
use crate::core::types::{XyChunk, XyToolSchema};

/// Streaming response from an LLM provider.
pub type XyStream = Pin<Box<dyn Stream<Item = Result<XyChunk, XyError>> + Send>>;

/// LLM provider contract.
///
/// Implementations connect to a remote API (OpenAI, Anthropic, etc.) and
/// produce a streaming response from a conversation history.
#[async_trait]
pub trait XyModel: Send + Sync {
    fn name(&self) -> &str;

    /// Generate a streaming response from AgentMessage history.
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

/// Tool contract — all tools must implement this.
///
/// The `execute` method receives a [`XyToolCtx`] which contains a
/// [`CancellationToken`]. Tools MUST:
/// 1. Check `ctx.cancel.is_cancelled()` at appropriate checkpoints.
/// 2. Return [`XyToolError::Aborted`] when cancellation is detected.
#[async_trait]
pub trait XyTool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> Value;
    async fn execute(&self, ctx: &XyToolCtx, args: Value) -> Result<String, XyToolError>;

    fn prompt_snippet(&self) -> Option<&str> {
        None
    }

    fn prompt_guidelines(&self) -> &[&str] {
        &[]
    }

    fn execution_mode(&self) -> ToolExecutionMode {
        ToolExecutionMode::Parallel
    }

    fn prepare_arguments(&self, args: Value) -> Value {
        args
    }
}
