//! Runtime boundary for tool execution.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::protocol::error::XyToolError;
use crate::protocol::lifecycle::XyEvent;
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
    /// Optional typed state-event uplink (atd13) for tools that mutate
    /// host-owned semantic state (e.g. `todo_*` publishing
    /// [`XyEvent::TodoUpdated`]). ReAct drains and re-emits verbatim into the
    /// run stream, so both in-process and attach clients see the event.
    /// `publish_state` is a no-op when unset.
    state_events: Option<mpsc::UnboundedSender<XyEvent>>,
    /// Workspace base directory for tool execution.
    ///
    /// File tools resolve relative paths against it and shell tools spawn in
    /// it. Defaults to the process cwd; the runtime MUST inject the frozen
    /// session workspace so multi-workspace hosts execute in the session's
    /// directory, not the server process directory.
    pub workspace: PathBuf,
}

impl XyToolCtx {
    pub fn new(call_id: impl Into<String>) -> Self {
        Self {
            call_id: call_id.into(),
            cancel: CancellationToken::new(),
            output_tx: None,
            state_events: None,
            workspace: fallback_workspace(),
        }
    }

    pub fn with_cancel(call_id: impl Into<String>, cancel: CancellationToken) -> Self {
        Self {
            call_id: call_id.into(),
            cancel,
            output_tx: None,
            state_events: None,
            workspace: fallback_workspace(),
        }
    }

    /// Attach a live output channel (c1255 bash / long-tool streaming).
    pub fn with_output_tx(mut self, tx: mpsc::Sender<String>) -> Self {
        self.output_tx = Some(tx);
        self
    }

    /// Attach the typed state-event uplink (atd13).
    pub fn with_state_event_tx(mut self, tx: mpsc::UnboundedSender<XyEvent>) -> Self {
        self.state_events = Some(tx);
        self
    }

    /// Publish a typed domain-state event from this tool call (atd13).
    ///
    /// Silent no-op without an uplink (unit tests / surfaces that don't consume
    /// state projections) — MUST NOT be load-bearing for SSOT persistence.
    pub fn publish_state(&self, event: XyEvent) {
        if let Some(tx) = &self.state_events {
            let _ = tx.send(event);
        }
    }

    /// Bind the execution workspace (session cwd). Relative paths in tool args
    /// and spawned shells resolve against it.
    pub fn with_workspace(mut self, workspace: impl AsRef<Path>) -> Self {
        self.workspace = workspace.as_ref().to_path_buf();
        self
    }
}

fn fallback_workspace() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
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
/// MCP / unknown names are forced to Barrier by the scheduler (`mcp__` / transition
/// `mcp-` / `mcp_` / legacy `mcp:` prefix / missing tool), independent of this trait value.
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

    #[test]
    fn xy_tool_ctx_workspace_defaults_to_process_cwd() {
        let ctx = XyToolCtx::new("ws-default");
        assert_eq!(
            ctx.workspace,
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        );
    }

    #[test]
    fn xy_tool_ctx_with_workspace_overrides() {
        let ctx = XyToolCtx::new("ws-explicit").with_workspace("/tmp/proj-a");
        assert_eq!(ctx.workspace, PathBuf::from("/tmp/proj-a"));
        let cancel = CancellationToken::new();
        let ctx = XyToolCtx::with_cancel("ws-explicit-2", cancel).with_workspace("./rel");
        assert_eq!(ctx.workspace, PathBuf::from("./rel"));
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
