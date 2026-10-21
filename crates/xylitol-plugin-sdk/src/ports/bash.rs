use std::path::PathBuf;

use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::ToolTimeout;

#[derive(Debug, Clone, Default)]
pub struct XyBashResult {
    pub output: String,
    pub exit_code: Option<i32>,
    pub cancelled: bool,
    pub timed_out: bool,
    pub truncated: bool,
    pub full_output_path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BashOutputSink {
    pub tx: mpsc::Sender<BashChunk>,
    pub cancel: CancellationToken,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BashChunk {
    pub bash_id: String,
    pub seq: u64,
    pub data: String,
}

#[derive(Default)]
pub struct BashExecOpts {
    pub cancel: Option<CancellationToken>,
    pub chunk_tx: Option<mpsc::Sender<Vec<u8>>>,
    pub timeout: ToolTimeout,
    pub cwd: Option<PathBuf>,
}

impl BashExecOpts {
    pub fn cancel_only(cancel: CancellationToken) -> Self {
        Self {
            cancel: Some(cancel),
            chunk_tx: None,
            timeout: ToolTimeout::Unlimited,
            cwd: None,
        }
    }
}

#[async_trait]
pub trait XyBashExecutor: Send + Sync {
    async fn execute(&self, command: &str, opts: BashExecOpts) -> XyBashResult;
}
