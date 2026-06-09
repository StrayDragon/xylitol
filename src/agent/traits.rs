use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use super::error::{XyError, XyToolError};
use super::types::{XyChunk, XyContent, XyToolSchema};

pub type XyStream = Pin<Box<dyn Stream<Item = Result<XyChunk, XyError>> + Send>>;

#[async_trait]
pub trait XyModel: Send + Sync {
    fn name(&self) -> &str;

    async fn generate_stream(
        &self,
        messages: Vec<XyContent>,
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

    /// Optional one-line prompt snippet for system prompt construction.
    /// Default returns the description.
    fn prompt_snippet(&self) -> Option<&str> {
        None
    }

    /// Optional prompt guidelines for system prompt.
    fn prompt_guidelines(&self) -> &[&str] {
        &[]
    }
}
