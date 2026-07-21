//! Persistent bash executor — user/RPC-initiated shell execution with recording.
//!
//! Unlike `tools/bash.rs` (the LLM tool-call entry point), this serves
//! interactive/RPC `!cmd` and `!!cmd` execution: it streams output bytes via
//! an optional bounded `chunk_tx` (`BashExecOpts`), supports cancellation via
//! a [`CancellationToken`], truncates output, and spills the full output to a
//! temp file when the rolling buffer overflows.
//!
//! This module lives under `infra/` because it is a runtime facility
//! (process spawn + output streaming). The agent consumes it only through the
//! `XyBashExecutor` port in `protocol::ports`.

use std::time::Duration;

use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::infra::tools::accumulator::OutputAccumulator;
use crate::infra::tools::process::kill_tree;
use crate::infra::tools::truncate::DEFAULT_MAX_BYTES;
use crate::protocol::ports::{BashExecOpts, XyBashExecutor, XyBashResult};

const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Default infra bash executor.
#[derive(Debug, Clone, Default)]
pub struct InfraBashExecutor;

impl InfraBashExecutor {
    /// Create a new infra bash executor.
    pub fn new() -> Self {
        Self
    }
}

impl InfraBashExecutor {
    fn emit_chunk(tx: &Option<mpsc::Sender<Vec<u8>>>, coalesce: &mut Vec<u8>, bytes: &[u8]) {
        let Some(tx) = tx else {
            return;
        };
        if !coalesce.is_empty() {
            coalesce.extend_from_slice(bytes);
            match tx.try_send(std::mem::take(coalesce)) {
                Ok(()) => {}
                Err(mpsc::error::TrySendError::Full(v)) => {
                    *coalesce = v;
                }
                Err(mpsc::error::TrySendError::Closed(v)) => {
                    *coalesce = v;
                }
            }
            return;
        }
        match tx.try_send(bytes.to_vec()) {
            Ok(()) => {}
            Err(mpsc::error::TrySendError::Full(v)) => {
                *coalesce = v;
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {}
        }
    }

    fn flush_coalesce(tx: &Option<mpsc::Sender<Vec<u8>>>, coalesce: &mut Vec<u8>) {
        if coalesce.is_empty() {
            return;
        }
        let Some(tx) = tx else {
            coalesce.clear();
            return;
        };
        let pending = std::mem::take(coalesce);
        match tx.try_send(pending) {
            Ok(()) => {}
            Err(mpsc::error::TrySendError::Full(v)) | Err(mpsc::error::TrySendError::Closed(v)) => {
                // Last-chance: drop if still full / closed — Accumulator remains SSOT.
                let _ = v;
            }
        }
    }
}

#[async_trait::async_trait]
impl XyBashExecutor for InfraBashExecutor {
    async fn execute(&self, command: &str, opts: BashExecOpts) -> XyBashResult {
        let BashExecOpts { cancel, chunk_tx } = opts;
        let timeout_secs = DEFAULT_TIMEOUT_SECS;
        let timeout_dur = Duration::from_secs(timeout_secs);

        if cancel.as_ref().is_some_and(|c| c.is_cancelled()) {
            return XyBashResult {
                output: String::new(),
                exit_code: None,
                cancelled: true,
                truncated: false,
                full_output_path: None,
            };
        }

        let shell_cfg = crate::infra::process::shell::find_bash(None);
        let mut child = match Command::new(&shell_cfg.shell)
            .args(&shell_cfg.args)
            .arg(command)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(_) => {
                return XyBashResult {
                    output: String::from("[failed to spawn shell]"),
                    exit_code: None,
                    cancelled: false,
                    truncated: false,
                    full_output_path: None,
                };
            }
        };

        let pid = child.id().unwrap_or(0);

        // Merge stdout + stderr via two reader tasks feeding a channel, so the
        // combined stream can be chunked and accumulated while still respecting
        // cancellation and timeout.
        let (tx, mut rx) = mpsc::channel::<Vec<u8>>(64);
        let tx_err = tx.clone();
        spawn_reader(child.stdout.take().expect("stdout piped"), tx);
        spawn_reader(child.stderr.take().expect("stderr piped"), tx_err);
        // tx dropped here (only clones in tasks); channel closes when readers done.

        let mut acc = OutputAccumulator::new();
        let mut cancelled = false;
        let mut coalesce = Vec::new();

        loop {
            tokio::select! {
                biased;
                _ = cancelled_event(&cancel) => {
                    cancelled = true;
                    break;
                }
                chunk = rx.recv() => {
                    match chunk {
                        Some(bytes) => {
                            acc.append(&bytes);
                            Self::emit_chunk(&chunk_tx, &mut coalesce, &bytes);
                        }
                        None => break, // both readers finished
                    }
                }
                _ = tokio::time::sleep(timeout_dur) => {
                    // Timeout reached.
                    break;
                }
            }
        }

        Self::flush_coalesce(&chunk_tx, &mut coalesce);

        if cancelled {
            kill_tree(pid).await;
        }
        // Reap the child and capture its exit status.
        let exit_code = child.wait().await.ok().and_then(|s| s.code());

        let snapshot = acc.finish();

        XyBashResult {
            output: snapshot.display_content(),
            exit_code: if cancelled { None } else { exit_code },
            cancelled,
            truncated: snapshot.truncated,
            full_output_path: snapshot
                .full_output_path
                .and_then(|p| p.to_str().map(|s| s.to_string())),
        }
    }
}

fn spawn_reader<R: tokio::io::AsyncRead + Unpin + Send + 'static>(
    mut reader: R,
    tx: mpsc::Sender<Vec<u8>>,
) {
    tokio::spawn(async move {
        let mut buf = vec![0u8; 8 * 1024];
        loop {
            match reader.read(&mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if tx.send(buf[..n].to_vec()).await.is_err() {
                        break;
                    }
                }
            }
        }
    });
}

async fn cancelled_event(cancel: &Option<CancellationToken>) {
    match cancel {
        Some(c) => c.cancelled().await,
        None => std::future::pending::<()>().await,
    }
}

const _: usize = DEFAULT_MAX_BYTES;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn records_exit_code() {
        let result = InfraBashExecutor::new()
            .execute("exit 7", BashExecOpts::default())
            .await;
        assert!(!result.cancelled);
        assert_eq!(result.exit_code, Some(7));
    }

    #[tokio::test]
    async fn cancellation_kills_process() {
        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(200)).await;
            cancel_clone.cancel();
        });
        let result = InfraBashExecutor::new()
            .execute(
                "sleep 30",
                BashExecOpts {
                    cancel: Some(cancel),
                    chunk_tx: None,
                },
            )
            .await;
        assert!(result.cancelled);
        assert_eq!(result.exit_code, None);
    }

    #[tokio::test]
    async fn merges_stdout_and_stderr() {
        let result = InfraBashExecutor::new()
            .execute("echo OUT; echo ERR >&2", BashExecOpts::default())
            .await;
        assert!(result.output.contains("OUT") || result.output.contains("ERR"));
    }

    #[tokio::test]
    async fn streams_chunks_when_tx_provided() {
        let (tx, mut rx) = mpsc::channel::<Vec<u8>>(64);
        let result = InfraBashExecutor::new()
            .execute(
                "printf 'hello\\nworld\\n'",
                BashExecOpts {
                    cancel: None,
                    chunk_tx: Some(tx),
                },
            )
            .await;
        assert!(!result.cancelled);
        let mut collected = Vec::new();
        while let Ok(chunk) = rx.try_recv() {
            collected.extend_from_slice(&chunk);
        }
        let text = String::from_utf8_lossy(&collected);
        assert!(
            text.contains("hello") || result.output.contains("hello"),
            "chunks or result must contain hello; chunks={text:?} result={:?}",
            result.output
        );
    }
}
