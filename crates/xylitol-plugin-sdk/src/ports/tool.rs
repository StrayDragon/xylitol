use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::error::XyToolError;
use crate::lifecycle::XyEvent;
use crate::message::AgentPart;

#[derive(Clone)]
pub struct XyToolCtx {
    pub call_id: String,
    pub cancel: CancellationToken,
    pub output_tx: Option<mpsc::Sender<String>>,
    state_events: Option<mpsc::UnboundedSender<XyEvent>>,
    pub workspace: PathBuf,
    dispatch: Option<Arc<dyn XyToolDispatch>>,
}

impl XyToolCtx {
    pub fn new(call_id: impl Into<String>) -> Self {
        Self {
            call_id: call_id.into(),
            cancel: CancellationToken::new(),
            output_tx: None,
            state_events: None,
            workspace: fallback_workspace(),
            dispatch: None,
        }
    }

    pub fn with_cancel(call_id: impl Into<String>, cancel: CancellationToken) -> Self {
        Self {
            call_id: call_id.into(),
            cancel,
            output_tx: None,
            state_events: None,
            workspace: fallback_workspace(),
            dispatch: None,
        }
    }

    pub fn with_output_tx(mut self, tx: mpsc::Sender<String>) -> Self {
        self.output_tx = Some(tx);
        self
    }

    pub fn with_state_event_tx(mut self, tx: mpsc::UnboundedSender<XyEvent>) -> Self {
        self.state_events = Some(tx);
        self
    }

    pub fn publish_state(&self, event: XyEvent) {
        if let Some(tx) = &self.state_events {
            let _ = tx.send(event);
        }
    }

    pub fn with_workspace(mut self, workspace: impl AsRef<Path>) -> Self {
        self.workspace = workspace.as_ref().to_path_buf();
        self
    }

    pub fn with_dispatch(mut self, dispatch: Arc<dyn XyToolDispatch>) -> Self {
        self.dispatch = Some(dispatch);
        self
    }

    pub fn dispatch(&self) -> Option<&Arc<dyn XyToolDispatch>> {
        self.dispatch.as_ref()
    }
}

fn fallback_workspace() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum XyBatchMode {
    Sequential,
    #[default]
    BarrierParallel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize)]
pub enum XyToolExecutionMode {
    Parallel,
    #[default]
    Sequential,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Default,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum XyToolExposure {
    #[default]
    Direct,
    ModelOnly,
    Codemode,
    Deferred,
    Hidden,
}

impl XyToolExposure {
    pub fn model_visible(self) -> bool {
        matches!(self, Self::Direct | Self::ModelOnly)
    }

    pub fn codemode_visible(self) -> bool {
        matches!(self, Self::Direct | Self::Codemode | Self::Deferred)
    }

    pub fn inline_declaration(self) -> bool {
        matches!(self, Self::Direct | Self::Codemode | Self::ModelOnly)
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct XyToolDescriptor {
    pub name: String,
    pub identifier: String,
    pub namespace: Option<String>,
    pub description: String,
    pub parameters: Value,
    pub output_schema: Option<Value>,
    pub exposure: XyToolExposure,
    pub guidelines: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum XyToolErrorKind {
    InvalidArgs,
    ExecutionFailed,
    PermissionDenied,
    Timeout,
    Aborted,
}

impl From<&XyToolError> for XyToolErrorKind {
    fn from(error: &XyToolError) -> Self {
        match error {
            XyToolError::InvalidArgs(_) => Self::InvalidArgs,
            XyToolError::ExecutionFailed(_) => Self::ExecutionFailed,
            XyToolError::PermissionDenied(_) => Self::PermissionDenied,
            XyToolError::Timeout(_) => Self::Timeout,
            XyToolError::Aborted => Self::Aborted,
        }
    }
}

#[derive(Debug, Clone)]
pub struct XyToolOutcome {
    pub parts: Vec<AgentPart>,
    pub details: Option<Value>,
    pub is_error: bool,
    pub error_kind: Option<XyToolErrorKind>,
}

impl XyToolOutcome {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            parts: vec![AgentPart::text(text)],
            details: None,
            is_error: false,
            error_kind: None,
        }
    }

    pub fn from_parts(parts: Vec<AgentPart>) -> Self {
        Self {
            parts,
            details: None,
            is_error: false,
            error_kind: None,
        }
    }

    pub fn text_value(&self) -> String {
        self.parts
            .iter()
            .filter_map(|part| match part {
                AgentPart::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

pub trait XyToolCatalog: Send + Sync {
    fn snapshot(&self) -> Vec<XyToolDescriptor>;
    fn describe(&self, name: &str) -> Option<XyToolDescriptor>;
    fn search(&self, query: &str, limit: usize) -> Vec<XyToolDescriptor>;
}

#[async_trait]
pub trait XyToolDispatch: Send + Sync {
    async fn call(
        &self,
        parent_call_id: &str,
        name: &str,
        args: Value,
    ) -> Result<XyToolOutcome, XyToolError>;
    fn catalog(&self) -> Arc<dyn XyToolCatalog>;

    fn nested_summary(&self) -> Option<Value> {
        None
    }
}

#[async_trait]
pub trait XyTool: Send + Sync {
    fn name(&self) -> &str;

    fn replaces(&self) -> &'static [&'static str] {
        &[]
    }

    fn description(&self) -> &str;
    fn parameters_schema(&self) -> Value;
    async fn execute_outcome(
        &self,
        ctx: &XyToolCtx,
        args: Value,
    ) -> Result<XyToolOutcome, XyToolError>;

    async fn execute(&self, ctx: &XyToolCtx, args: Value) -> Result<String, XyToolError> {
        Ok(self.execute_outcome(ctx, args).await?.text_value())
    }

    fn prompt_snippet(&self) -> Option<&str> {
        None
    }

    fn prompt_guidelines(&self) -> &[&str] {
        &[]
    }

    fn execution_mode(&self) -> XyToolExecutionMode {
        XyToolExecutionMode::Sequential
    }

    fn prepare_arguments(&self, args: Value) -> Value {
        args
    }

    fn exposure(&self) -> XyToolExposure {
        XyToolExposure::Direct
    }

    fn namespace(&self) -> Option<&str> {
        None
    }

    fn output_schema(&self) -> Option<Value> {
        None
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

        async fn execute_outcome(
            &self,
            _ctx: &XyToolCtx,
            args: Value,
        ) -> Result<XyToolOutcome, XyToolError> {
            Ok(XyToolOutcome::text(format!("executed with: {args}")))
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

    #[test]
    fn tool_error_kind_maps_and_serializes() {
        let timeout =
            XyToolErrorKind::from(&XyToolError::Timeout(std::time::Duration::from_secs(1)));
        assert_eq!(timeout, XyToolErrorKind::Timeout);
        assert_eq!(serde_json::to_string(&timeout).unwrap(), "\"timeout\"");
        assert_eq!(
            XyToolErrorKind::from(&XyToolError::Aborted),
            XyToolErrorKind::Aborted
        );
    }

    #[tokio::test]
    async fn mock_tool_defaults() {
        let tool = MockTool;
        let args = json!("raw");
        assert_eq!(tool.prepare_arguments(args.clone()), args);
    }
}
