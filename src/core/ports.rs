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

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde_json::json;

    // ── XyToolCtx ───────────────────────────────────────────────────

    #[test]
    fn xy_tool_ctx_new() {
        let ctx = XyToolCtx::new("call-1");
        assert_eq!(ctx.call_id, "call-1");
        assert!(!ctx.cancel.is_cancelled());
    }

    #[test]
    fn xy_tool_ctx_with_cancel() {
        let cancel = CancellationToken::new();
        let ctx = XyToolCtx::with_cancel("call-2", cancel.clone());
        assert_eq!(ctx.call_id, "call-2");
        cancel.cancel();
        assert!(ctx.cancel.is_cancelled());
    }

    #[test]
    fn xy_tool_ctx_cancel_not_cancelled_by_default() {
        let ctx = XyToolCtx::new("call-3");
        assert!(!ctx.cancel.is_cancelled());
    }

    // ── ToolExecutionMode ───────────────────────────────────────────

    #[test]
    fn tool_execution_mode_default_is_parallel() {
        assert_eq!(ToolExecutionMode::default(), ToolExecutionMode::Parallel);
    }

    #[test]
    fn tool_execution_mode_serialize() {
        let json = serde_json::to_string(&ToolExecutionMode::Parallel).unwrap();
        assert_eq!(json, "\"Parallel\"");
        let json = serde_json::to_string(&ToolExecutionMode::Sequential).unwrap();
        assert_eq!(json, "\"Sequential\"");
    }

    #[test]
    fn tool_execution_mode_eq() {
        assert_eq!(ToolExecutionMode::Parallel, ToolExecutionMode::Parallel);
        assert_ne!(ToolExecutionMode::Parallel, ToolExecutionMode::Sequential);
    }

    // ── Mock XyTool ─────────────────────────────────────────────────

    struct MockTool;

    #[async_trait]
    impl XyTool for MockTool {
        fn name(&self) -> &str {
            "mock_tool"
        }

        fn description(&self) -> &str {
            "A mock tool for testing"
        }

        fn parameters_schema(&self) -> Value {
            json!({
                "type": "object",
                "properties": {
                    "input": {"type": "string"}
                }
            })
        }

        async fn execute(&self, _ctx: &XyToolCtx, args: Value) -> Result<String, XyToolError> {
            Ok(format!("executed with: {args}"))
        }
    }

    #[tokio::test]
    async fn mock_tool_contract() {
        let tool = MockTool;
        assert_eq!(tool.name(), "mock_tool");
        assert_eq!(tool.description(), "A mock tool for testing");
        assert!(tool.prompt_snippet().is_none());
        assert!(tool.prompt_guidelines().is_empty());
        assert_eq!(tool.execution_mode(), ToolExecutionMode::Parallel);

        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");

        let ctx = XyToolCtx::new("call-1");
        let result = tool.execute(&ctx, json!({"input": "hello"})).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "executed with: {\"input\":\"hello\"}");
    }

    #[tokio::test]
    async fn mock_tool_defaults() {
        let tool = MockTool;
        // Default prepare_arguments should pass through
        let args = json!("raw");
        assert_eq!(tool.prepare_arguments(args.clone()), args);
    }

    // ── Streaming type ──────────────────────────────────────────────

    #[test]
    fn xy_stream_type_is_send() {
        // Compile-time check: XyStream must be Send
        fn assert_send<T: Send>() {}
        assert_send::<XyStream>();
    }
}
