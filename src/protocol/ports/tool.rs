//! Runtime boundary for tool execution.

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::protocol::error::XyToolError;
use crate::protocol::message::AgentPart;

/// Context passed to tool execution.
#[derive(Clone)]
pub struct XyToolCtx {
    /// Unique identifier for this tool call.
    pub call_id: String,
    /// Cancellation token — tools should check this and abort if cancelled.
    pub cancel: CancellationToken,
    /// Optional live output uplink for [`crate::protocol::lifecycle::XyEvent::ToolExecutionUpdate`].
    ///
    /// Long-running tools (e.g. bash) SHOULD send progressive chunks here while
    /// executing. ReAct drains this channel and emits Update events. `None` for
    /// tools that only report a final result.
    pub output_tx: Option<mpsc::Sender<String>>,
}

impl XyToolCtx {
    pub fn new(call_id: impl Into<String>) -> Self {
        Self {
            call_id: call_id.into(),
            cancel: CancellationToken::new(),
            output_tx: None,
        }
    }

    pub fn with_cancel(call_id: impl Into<String>, cancel: CancellationToken) -> Self {
        Self {
            call_id: call_id.into(),
            cancel,
            output_tx: None,
        }
    }

    /// Attach a live output channel (c1255 bash / long-tool streaming).
    pub fn with_output_tx(mut self, tx: mpsc::Sender<String>) -> Self {
        self.output_tx = Some(tx);
        self
    }
}

/// Session / config tool-batch scheduling mode (c1545).
///
/// Distinct from per-tool [`XyToolExecutionMode`] (concurrency class).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum XyBatchMode {
    /// Source-order one-by-one await.
    Sequential,
    /// Consecutive ParallelSafe tools fan out; Barrier tools flush then run alone.
    /// Product default (c1610); was experimental opt-in under c1545.
    #[default]
    BarrierParallel,
}

/// Per-tool concurrency class used when batch mode is [`XyBatchMode::BarrierParallel`].
///
/// - [`Self::Parallel`] = ParallelSafe (may share a parallel window)
/// - [`Self::Sequential`] = Barrier (flush window, then run alone)
///
/// Default for undeclared / self-registered tools is Barrier ([`Self::Sequential`]).
/// Built-in read-family tools opt into ParallelSafe via [`XyTool::execution_mode`].
/// MCP / unknown names are forced to Barrier by the scheduler (`mcp:` prefix / missing
/// tool), independent of this trait value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize)]
pub enum XyToolExecutionMode {
    /// ParallelSafe — may run concurrently with other ParallelSafe tools in a window.
    Parallel,
    /// Barrier — must not share a parallel window; flush first, then run alone.
    #[default]
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

    /// Multimodal tool result (text + images). Default wraps [`Self::execute`] as a text part.
    ///
    /// `read` overrides this for image files (c1155 / t21). React MUST prefer this over
    /// wrapping `execute` alone so Image parts reach the provider.
    async fn execute_as_parts(
        &self,
        ctx: &XyToolCtx,
        args: Value,
    ) -> Result<Vec<AgentPart>, XyToolError> {
        Ok(vec![AgentPart::text(self.execute(ctx, args).await?)])
    }

    fn prompt_snippet(&self) -> Option<&str> {
        None
    }

    fn prompt_guidelines(&self) -> &[&str] {
        &[]
    }

    /// Per-tool concurrency class (ParallelSafe / Barrier). See [`XyToolExecutionMode`].
    ///
    /// Default is Barrier ([`XyToolExecutionMode::Sequential`]) for undeclared tools.
    fn execution_mode(&self) -> XyToolExecutionMode {
        XyToolExecutionMode::Sequential
    }

    fn prepare_arguments(&self, args: Value) -> Value {
        args
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use serde_json::json;

    use super::*;

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

    #[test]
    fn tool_execution_mode_default_is_barrier() {
        assert_eq!(
            XyToolExecutionMode::default(),
            XyToolExecutionMode::Sequential
        );
    }

    #[test]
    fn batch_mode_default_is_barrier_parallel() {
        // Product default after c1610.
        assert_eq!(XyBatchMode::default(), XyBatchMode::BarrierParallel);
    }

    #[test]
    fn batch_mode_serde_snake_case() {
        assert_eq!(
            serde_json::to_string(&XyBatchMode::Sequential).unwrap(),
            "\"sequential\""
        );
        assert_eq!(
            serde_json::to_string(&XyBatchMode::BarrierParallel).unwrap(),
            "\"barrier_parallel\""
        );
        assert_eq!(
            serde_json::from_str::<XyBatchMode>("\"barrier_parallel\"").unwrap(),
            XyBatchMode::BarrierParallel
        );
    }

    #[test]
    fn tool_execution_mode_serialize() {
        let json = serde_json::to_string(&XyToolExecutionMode::Parallel).unwrap();
        assert_eq!(json, "\"Parallel\"");
        let json = serde_json::to_string(&XyToolExecutionMode::Sequential).unwrap();
        assert_eq!(json, "\"Sequential\"");
    }

    #[test]
    fn tool_execution_mode_eq() {
        assert_eq!(XyToolExecutionMode::Parallel, XyToolExecutionMode::Parallel);
        assert_ne!(
            XyToolExecutionMode::Parallel,
            XyToolExecutionMode::Sequential
        );
    }

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
        assert_eq!(tool.execution_mode(), XyToolExecutionMode::Sequential);

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
}
