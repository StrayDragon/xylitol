//! Typed built-in tools: deserialize once, then run typed execute.
//!
//! Trait boundary for MCP / dynamic tools stays [`serde_json::Value`] + [`XyTool`].
//! Built-ins MAY implement [`TypedTool`] and get [`XyTool`] via the blanket impl.

use std::time::Duration;

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::args::parse_tool_args;
use crate::protocol::error::XyToolError;
use crate::protocol::message::AgentPart;
use crate::protocol::ports::{XyTool, XyToolCtx, XyToolExecutionMode};

/// Default wall-clock bound for file-system tools (c2425). Local FS work is
/// normally sub-second; the bound only fires on hung mounts (NFS/FUSE).
pub const FS_TOOL_TIMEOUT_SECS: u64 = 30;

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

    /// Optional wall-clock bound applied by the blanket [`XyTool`] impl.
    ///
    /// `None` (default) leaves the wait unbounded at this layer — command
    /// tools arm their own `ToolTimeout` from args instead. File-system tools
    /// override this to guarantee bounded waits on hung mounts.
    fn wait_bound(&self) -> Option<Duration> {
        None
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
        match self.wait_bound() {
            None => self.execute_typed(ctx, typed).await,
            Some(bound) => tokio::time::timeout(bound, self.execute_typed(ctx, typed))
                .await
                .unwrap_or_else(|_| {
                    Err(XyToolError::Timeout(
                        crate::protocol::ToolTimeout::After(bound)
                            .duration()
                            .expect("bound is a duration"),
                    ))
                }),
        }
    }

    async fn execute_as_parts(
        &self,
        ctx: &XyToolCtx,
        args: Value,
    ) -> Result<Vec<AgentPart>, XyToolError> {
        let typed = parse_tool_args(args)?;
        match self.wait_bound() {
            None => self.execute_as_parts_typed(ctx, typed).await,
            Some(bound) => tokio::time::timeout(bound, self.execute_as_parts_typed(ctx, typed))
                .await
                .unwrap_or_else(|_| {
                    Err(XyToolError::Timeout(
                        crate::protocol::ToolTimeout::After(bound)
                            .duration()
                            .expect("bound is a duration"),
                    ))
                }),
        }
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
