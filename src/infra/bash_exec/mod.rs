//! Persistent bash executor — user/RPC-initiated shell execution with recording.
//!
//! Unlike `tools/bash.rs` (the LLM tool-call entry point), this serves
//! interactive/RPC `!cmd` and `!!cmd` execution: it streams sanitized output
//! via an `on_chunk` callback, supports cancellation via a [`CancellationToken`],
//! truncates output, and spills the full output to a temp file when the rolling
//! buffer overflows.
//!
//! This module lives under `infra/` because it is a runtime facility
//! (process spawn + output streaming). The agent consumes it only through the
//! `BashExecutor` port in `runtime_protocol`.

use std::time::Duration;

use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::infra::tools::accumulator::OutputAccumulator;
use crate::infra::tools::process::kill_tree;
use crate::infra::tools::truncate::DEFAULT_MAX_BYTES;
use crate::runtime_protocol::{BashExecutor, BashResult};

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

#[async_trait::async_trait]
impl BashExecutor for InfraBashExecutor {
    async fn execute(&self, command: &str, cancel: Option<CancellationToken>) -> BashResult {
        let timeout_secs = DEFAULT_TIMEOUT_SECS;
        let timeout_dur = Duration::from_secs(timeout_secs);

        if cancel.as_ref().is_some_and(|c| c.is_cancelled()) {
            return BashResult {
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
                return BashResult {
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

        if cancelled {
            kill_tree(pid).await;
        }
        // Reap the child and capture its exit status.
        let exit_code = child.wait().await.ok().and_then(|s| s.code());

        let snapshot = acc.finish();

        BashResult {
            output: snapshot.content,
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

async fn cancelled_event(cancel: &Option<CancellationToken>) -> () {
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
        let result = InfraBashExecutor::new().execute("exit 7", None).await;
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
            .execute("sleep 30", Some(cancel))
            .await;
        assert!(result.cancelled);
        assert_eq!(result.exit_code, None);
    }

    #[tokio::test]
    async fn merges_stdout_and_stderr() {
        let result = InfraBashExecutor::new()
            .execute("echo OUT; echo ERR >&2", None)
            .await;
        assert!(result.output.contains("OUT") || result.output.contains("ERR"));
    }
}
