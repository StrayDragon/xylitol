use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub(crate) enum XyError {
    #[error("provider error: {0}")]
    Provider(#[source] anyhow::Error),
    #[error("tool error: {0}")]
    Tool(#[from] XyToolError),
    #[error("session error: {0}")]
    Session(#[source] anyhow::Error),
    #[error("max iterations reached ({0})")]
    MaxIterations(usize),
    #[error("agent config error: {0}")]
    Config(String),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum XyToolError {
    #[error("invalid arguments: {0}")]
    InvalidArgs(String),
    #[error("execution failed: {0}")]
    ExecutionFailed(#[source] anyhow::Error),
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    #[error("timeout after {0:?}")]
    Timeout(Duration),
}
