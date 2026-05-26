use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;

use super::error::{XyError, XyToolError};
use super::types::{XyChunk, XyContent, XyToolSchema};

pub(crate) type XyStream = Pin<Box<dyn Stream<Item = Result<XyChunk, XyError>> + Send>>;

#[async_trait]
pub(crate) trait XyModel: Send + Sync {
    fn name(&self) -> &str;

    async fn generate_stream(
        &self,
        messages: Vec<XyContent>,
        tools: &[XyToolSchema],
        stream: bool,
    ) -> Result<XyStream, XyError>;
}

/// Minimal context passed to tool execution (replaces adk ToolContext).
pub(crate) struct XyToolCtx {
    pub call_id: String,
}

#[async_trait]
pub(crate) trait XyTool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> Value;
    async fn execute(&self, ctx: &XyToolCtx, args: Value) -> Result<String, XyToolError>;
}
