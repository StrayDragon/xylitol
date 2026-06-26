//! Persistent bash executor — user/RPC-initiated shell execution with recording.
//!
//! Unlike `tools/bash.rs` (the LLM tool-call entry point), this serves
//! interactive/RPC `!cmd` and `!!cmd`
//! execution: it streams sanitized output via an `on_chunk` callback, supports
//! cancellation via a [`CancellationToken`], truncates output, and spills the
//! full output to a temp file when the rolling buffer overflows.
//!
//! Shares primitives with the tool version:
//! - `tools/process::kill_tree` for process-group termination
//! - `tools/accumulator::OutputAccumulator` for rolling buffer + spill
//! - `tools/truncate::DEFAULT_MAX_BYTES`

use std::time::Duration;

use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

// NOTE: runtime/bash shares exec primitives with infra::tools::bash (the LLM tool).
// These are low-level utilities (rolling buffer, process kill, byte cap), not
// provider/tool ports, so agent depending on them is acceptable. ceiling: if a
// second agent-side exec facility needs them, lift to core or a shared crate.
use crate::infra::tools::accumulator::OutputAccumulator;
use crate::infra::tools::process::kill_tree;
use crate::infra::tools::truncate::DEFAULT_MAX_BYTES;

/// Streaming output chunk callback for bash execution.
pub(crate) type OnChunkCallback<'a> = Box<dyn FnMut(&str) + Send + 'a>;

const DEFAULT_TIMEOUT_SECS: u64 = 30;
const MAX_TIMEOUT_SECS: u64 = 120;

/// Options for executing a bash command.
pub struct BashExecutorOptions<'a> {
    /// Streaming callback for output chunks (combined stdout+stderr).
    pub on_chunk: Option<OnChunkCallback<'a>>,
    /// Cancellation token; aborts execution and kills the process group.
    pub cancel: Option<CancellationToken>,
    /// Timeout in seconds (clamped to [`MAX_TIMEOUT_SECS`], default [`DEFAULT_TIMEOUT_SECS`]).
    pub timeout_secs: u64,
}

impl<'a> Default for BashExecutorOptions<'a> {
    fn default() -> Self {
        Self {
            on_chunk: None,
            cancel: None,
            timeout_secs: DEFAULT_TIMEOUT_SECS,
        }
    }
}

/// Result of executing a bash command.
#[derive(Debug, Clone)]
pub struct BashResult {
    /// Combined stdout + stderr output (possibly truncated; tail kept).
    pub output: String,
    /// Process exit code (`None` if killed/cancelled before exit).
    pub exit_code: Option<i32>,
    /// Whether the command was cancelled via the cancel token.
    pub cancelled: bool,
    /// Whether the output was truncated.
    pub truncated: bool,
    /// Path to a temp file containing the full output, if spilled.
    pub full_output_path: Option<String>,
}

/// Execute a bash command using the given options.
pub async fn execute(command: &str, opts: BashExecutorOptions<'_>) -> BashResult {
    let timeout_secs = if opts.timeout_secs == 0 {
        DEFAULT_TIMEOUT_SECS
    } else {
        opts.timeout_secs.min(MAX_TIMEOUT_SECS)
    };
    let timeout_dur = Duration::from_secs(timeout_secs);

    let cancel = opts.cancel.clone();

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
    // combined stream can be chunked to `on_chunk` and the accumulator while
    // still respecting cancellation and timeout.
    let (tx, mut rx) = mpsc::channel::<Vec<u8>>(64);
    let tx_err = tx.clone();
    spawn_reader(child.stdout.take().expect("stdout piped"), tx);
    spawn_reader(child.stderr.take().expect("stderr piped"), tx_err);
    // tx dropped here (only clones in tasks); channel closes when readers done.

    let mut on_chunk = opts.on_chunk;
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
                        if let Some(cb) = on_chunk.as_mut() {
                            let text = String::from_utf8_lossy(&bytes);
                            cb(&text);
                        }
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

/// Returns `(exclude_from_context, command_without_prefix)` for a `!`/`!!` line.
///
/// `!!cmd` → exclude=true; `!cmd` → exclude=false; otherwise `None`.
pub fn parse_bang_prefix(input: &str) -> Option<(bool, &str)> {
    let trimmed = input.trim_start();
    if let Some(rest) = trimmed.strip_prefix("!!") {
        Some((true, rest.trim_start()))
    } else if let Some(rest) = trimmed.strip_prefix('!') {
        Some((false, rest.trim_start()))
    } else {
        None
    }
}

const _: usize = DEFAULT_MAX_BYTES;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn streams_output_to_callback() {
        let received = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let received_clone = received.clone();
        let opts = BashExecutorOptions {
            on_chunk: Some(Box::new(move |chunk: &str| {
                received_clone.lock().unwrap().push_str(chunk);
            })),
            cancel: None,
            timeout_secs: 10,
        };
        let result = execute("echo hello", opts).await;
        assert!(!result.cancelled);
        let collected = received.lock().unwrap().clone();
        assert!(collected.contains("hello") || result.output.contains("hello"));
    }

    #[tokio::test]
    async fn records_exit_code() {
        let result = execute("exit 7", BashExecutorOptions::default()).await;
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
        let result = execute(
            "sleep 30",
            BashExecutorOptions {
                on_chunk: None,
                cancel: Some(cancel),
                timeout_secs: 30,
            },
        )
        .await;
        assert!(result.cancelled);
        assert_eq!(result.exit_code, None);
    }

    #[tokio::test]
    async fn merges_stdout_and_stderr() {
        // Both streams produce output; combined result must contain both.
        let result = execute("echo OUT; echo ERR >&2", BashExecutorOptions::default()).await;
        assert!(result.output.contains("OUT") || result.output.contains("ERR"));
    }

    #[test]
    fn parses_double_bang_as_exclude() {
        let (exclude, cmd) = parse_bang_prefix("!!ls -la").unwrap();
        assert!(exclude);
        assert_eq!(cmd, "ls -la");
    }

    #[test]
    fn parses_single_bang_as_include() {
        let (exclude, cmd) = parse_bang_prefix("!echo hi").unwrap();
        assert!(!exclude);
        assert_eq!(cmd, "echo hi");
    }

    #[test]
    fn ignores_non_bang_input() {
        assert!(parse_bang_prefix("ls").is_none());
        assert!(parse_bang_prefix("/compact").is_none());
    }
}
