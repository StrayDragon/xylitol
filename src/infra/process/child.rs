//! Reliable child process waiting with pipe drain protection.
//!
//! A process may `exit` while a detached descendant keeps stdout/stderr pipes
//! open. We must not resolve while output is still arriving. After `exit`,
//! wait for pipes to fall idle before finalizing.

use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::process::Child;
use tokio::sync::mpsc;

const EXIT_STDIO_GRACE_MS: u64 = 100;

enum ChildEvent {
    Exited(Option<i32>),
    Data,
}

/// Wait for a child process to terminate without hanging on inherited stdio.
///
/// Reads all available data from pipes, then waits for the grace period
/// after the child exits to let any lingering descendants flush their output.
///
/// Returns the exit code, or `None` if the child was killed.
pub async fn wait_for_child(mut child: Child) -> Option<i32> {
    let (tx, mut rx) = mpsc::channel::<ChildEvent>(64);

    // Take the pipes before spawning
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    // Read stdout
    if let Some(mut reader) = stdout {
        let data_tx = tx.clone();
        tokio::spawn(async move {
            let mut buf = vec![0u8; 4096];
            loop {
                match reader.read(&mut buf).await {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        if data_tx.send(ChildEvent::Data).await.is_err() {
                            break;
                        }
                    }
                }
            }
        });
    }

    // Read stderr
    if let Some(mut reader) = stderr {
        let data_tx = tx.clone();
        tokio::spawn(async move {
            let mut buf = vec![0u8; 4096];
            loop {
                match reader.read(&mut buf).await {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        if data_tx.send(ChildEvent::Data).await.is_err() {
                            break;
                        }
                    }
                }
            }
        });
    }

    // Wait for child exit in another task
    let exit_tx = tx.clone();
    tokio::spawn(async move {
        let status = child.wait().await;
        let code = status.ok().and_then(|s| s.code());
        let _ = exit_tx.send(ChildEvent::Exited(code)).await;
    });

    // Drop our sender so channel closes when all tasks finish
    drop(tx);

    let mut exit_code: Option<i32> = None;
    let mut exited = false;

    loop {
        tokio::select! {
            event = rx.recv() => {
                match event {
                    Some(ChildEvent::Exited(code)) => {
                        exit_code = code;
                        exited = true;
                        // Don't break yet — wait for pipe drain
                    }
                    Some(ChildEvent::Data) => {
                        // Data still arriving — reset idle timer
                    }
                    None => {
                        // All senders dropped — pipes closed
                        break;
                    }
                }
            }
            _ = async {
                if exited {
                    tokio::time::sleep(Duration::from_millis(EXIT_STDIO_GRACE_MS)).await;
                } else {
                    std::future::pending::<()>().await;
                }
            } => {
                // Grace period elapsed after exit with no more data
                break;
            }
        }
    }

    exit_code
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::process::Command;

    #[tokio::test]
    async fn test_wait_for_child_basic() {
        let child = Command::new("echo")
            .arg("hello")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("failed to spawn echo");

        let code = wait_for_child(child).await;
        assert_eq!(code, Some(0));
    }

    #[tokio::test]
    async fn test_wait_for_child_exit_code() {
        let child = Command::new("sh")
            .args(["-c", "exit 42"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("failed to spawn sh");

        let code = wait_for_child(child).await;
        assert_eq!(code, Some(42));
    }

    #[tokio::test]
    async fn test_wait_for_child_with_output() {
        let child = Command::new("sh")
            .args(["-c", "echo outdata; echo errdata >&2"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("failed to spawn sh");

        let code = wait_for_child(child).await;
        assert_eq!(code, Some(0));
    }
}
