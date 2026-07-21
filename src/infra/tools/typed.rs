//! Typed built-in tools: deserialize once, then run typed execute.
//!
//! Trait boundary for MCP / dynamic tools stays [`serde_json::Value`] + [`XyTool`].
//! Built-ins MAY implement [`TypedTool`] and get [`XyTool`] via the blanket impl.

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::args::parse_tool_args;
use crate::protocol::error::XyToolError;
use crate::protocol::message::AgentPart;
use crate::protocol::ports::{XyTool, XyToolCtx, XyToolExecutionMode};

/// Built-in tool with typed args. Prefer this over manual [`XyTool`] when args
/// are a fixed `Deserialize` struct (see `ls` / `find`).
#[async_trait]
pub trait TypedTool: Send + Sync {
    type Args: DeserializeOwned + Send;

    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> Value;

    async fn execute_typed(&self, ctx: &XyToolCtx, args: Self::Args)
    -> Result<String, XyToolError>;

    async fn execute_as_parts_typed(
        &self,
        ctx: &XyToolCtx,
        args: Self::Args,
    ) -> Result<Vec<AgentPart>, XyToolError> {
        Ok(vec![AgentPart::text(self.execute_typed(ctx, args).await?)])
    }

    fn prompt_snippet(&self) -> Option<&str> {
        None
    }

    fn prompt_guidelines(&self) -> &[&str] {
        &[]
    }

    fn execution_mode(&self) -> XyToolExecutionMode {
        XyToolExecutionMode::Parallel
    }

    fn prepare_arguments(&self, args: Value) -> Value {
        args
    }
}

#[async_trait]
impl<T> XyTool for T
where
    T: TypedTool,
{
    fn name(&self) -> &str {
        TypedTool::name(self)
    }

    fn description(&self) -> &str {
        TypedTool::description(self)
    }

    fn parameters_schema(&self) -> Value {
        TypedTool::parameters_schema(self)
    }

    async fn execute(&self, ctx: &XyToolCtx, args: Value) -> Result<String, XyToolError> {
        let typed = parse_tool_args(args)?;
        self.execute_typed(ctx, typed).await
    }

    async fn execute_as_parts(
        &self,
        ctx: &XyToolCtx,
        args: Value,
    ) -> Result<Vec<AgentPart>, XyToolError> {
        let typed = parse_tool_args(args)?;
        self.execute_as_parts_typed(ctx, typed).await
    }

    fn prompt_snippet(&self) -> Option<&str> {
        TypedTool::prompt_snippet(self)
    }

    fn prompt_guidelines(&self) -> &[&str] {
        TypedTool::prompt_guidelines(self)
    }

    fn execution_mode(&self) -> XyToolExecutionMode {
        TypedTool::execution_mode(self)
    }

    fn prepare_arguments(&self, args: Value) -> Value {
        TypedTool::prepare_arguments(self, args)
    }
}
