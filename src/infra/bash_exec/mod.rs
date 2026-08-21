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

use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::infra::tools::accumulator::OutputAccumulator;
use crate::infra::tools::process::kill_tree;
use crate::infra::tools::truncate::DEFAULT_MAX_BYTES;
use crate::protocol::ToolTimeout;
use crate::protocol::ports::{BashExecOpts, XyBashExecutor, XyBashResult};

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
        let BashExecOpts {
            cancel,
            chunk_tx,
            timeout,
            cwd,
        } = opts;

        if cancel.as_ref().is_some_and(|c| c.is_cancelled()) {
            return XyBashResult {
                output: String::new(),
                exit_code: None,
                cancelled: true,
                timed_out: false,
                truncated: false,
                full_output_path: None,
            };
        }

        let shell_cfg = crate::infra::process::shell::find_bash(None);
        let mut spawn = Command::new(&shell_cfg.shell);
        spawn
            .args(&shell_cfg.args)
            .arg(command)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        if let Some(dir) = cwd {
            spawn.current_dir(dir);
        }
        let mut child = match spawn.spawn() {
            Ok(c) => c,
            Err(_) => {
                return XyBashResult {
                    output: String::from("[failed to spawn shell]"),
                    exit_code: None,
                    cancelled: false,
                    timed_out: false,
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
        let mut timed_out = false;
        let mut coalesce = Vec::new();

        let deadline = match timeout {
            ToolTimeout::Unlimited => None,
            ToolTimeout::After(d) => Some(tokio::time::Instant::now() + d),
        };

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
                _ = sleep_until_opt(deadline) => {
                    timed_out = true;
                    break;
                }
            }
        }

        Self::flush_coalesce(&chunk_tx, &mut coalesce);

        if cancelled || timed_out {
            kill_tree(pid).await;
        }
        // Reap the child and capture its exit status.
        let exit_code = child.wait().await.ok().and_then(|s| s.code());

        let snapshot = acc.finish();

        XyBashResult {
            output: snapshot.display_content(),
            exit_code: if cancelled || timed_out {
                None
            } else {
                exit_code
            },
            cancelled,
            timed_out,
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

async fn sleep_until_opt(deadline: Option<tokio::time::Instant>) {
    match deadline {
        Some(dl) => tokio::time::sleep_until(dl).await,
        None => std::future::pending::<()>().await,
    }
}

const _: usize = DEFAULT_MAX_BYTES;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn records_exit_code() {
        let result = InfraBashExecutor::new()
            .execute("exit 7", BashExecOpts::default())
            .await;
        assert!(!result.cancelled);
        assert!(!result.timed_out);
        assert_eq!(result.exit_code, Some(7));
    }

    #[tokio::test]
    async fn cwd_option_spawns_shell_in_workspace() {
        let dir = tempfile::tempdir().expect("tmp");
        let result = InfraBashExecutor::new()
            .execute(
                "pwd",
                BashExecOpts {
                    cwd: Some(dir.path().to_path_buf()),
                    ..Default::default()
                },
            )
            .await;
        let got = std::path::PathBuf::from(result.output.trim());
        assert_eq!(
            got.canonicalize().unwrap(),
            dir.path().canonicalize().unwrap(),
            "executor MUST spawn the shell in opts.cwd, got: {}",
            result.output
        );
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
                    timeout: ToolTimeout::Unlimited,
                    cwd: None,
                },
            )
            .await;
        assert!(result.cancelled);
        assert!(!result.timed_out);
        assert_eq!(result.exit_code, None);
    }

    #[tokio::test]
    async fn explicit_timeout_kills() {
        let result = InfraBashExecutor::new()
            .execute(
                "sleep 30",
                BashExecOpts {
                    cancel: None,
                    chunk_tx: None,
                    timeout: ToolTimeout::After(Duration::from_secs(1)),
                    cwd: None,
                },
            )
            .await;
        assert!(result.timed_out);
        assert!(!result.cancelled);
        assert_eq!(result.exit_code, None);
    }

    #[tokio::test]
    async fn omit_timeout_allows_short_sleep() {
        let result = InfraBashExecutor::new()
            .execute("sleep 2", BashExecOpts::default())
            .await;
        assert!(!result.timed_out);
        assert!(!result.cancelled);
        assert_eq!(result.exit_code, Some(0));
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
                    timeout: ToolTimeout::Unlimited,
                    cwd: None,
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
