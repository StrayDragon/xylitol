pub const CREDENTIAL_VARS: &[&str] = &["XYLITOL_SERVE_TOKEN", "XYLITOL_AUTH_TOKEN"];

pub fn scrub_tokio_command(command: &mut tokio::process::Command) {
    for var in CREDENTIAL_VARS {
        command.env_remove(*var);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandCaptureError {
    pub kind: &'static str,
    pub message: String,
}

impl CommandCaptureError {
    fn new(kind: &'static str, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

fn kill_process_tree(pid: u32) {
    if pid == 0 {
        return;
    }

    #[cfg(unix)]
    {
        let pid = pid as i32;
        unsafe { libc::kill(-pid, libc::SIGKILL) };
        unsafe { libc::kill(pid, libc::SIGKILL) };
    }

    #[cfg(windows)]
    {
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
}

pub async fn run_command_capture(
    command: &str,
    timeout: std::time::Duration,
    max_bytes: usize,
) -> Result<String, CommandCaptureError> {
    use tokio::io::AsyncReadExt;

    let mut spawn = tokio::process::Command::new("sh");
    spawn
        .arg("-c")
        .arg(command)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    #[cfg(unix)]
    spawn.process_group(0);
    scrub_tokio_command(&mut spawn);
    let mut child = spawn
        .spawn()
        .map_err(|error| CommandCaptureError::new("spawn_failed", error.to_string()))?;
    let child_pid = child.id().unwrap_or(0);
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| CommandCaptureError::new("io_error", "stdout pipe missing"))?;
    let mut reader = tokio::io::BufReader::new(stdout);
    let mut buffer = Vec::new();
    let wait = async {
        let mut chunk = [0u8; 8192];
        loop {
            let read = reader
                .read(&mut chunk)
                .await
                .map_err(|error| CommandCaptureError::new("io_error", error.to_string()))?;
            if read == 0 {
                break;
            }
            if buffer.len() < max_bytes {
                let take = (max_bytes - buffer.len()).min(read);
                buffer.extend_from_slice(&chunk[..take]);
            }
        }
        child
            .wait()
            .await
            .map_err(|error| CommandCaptureError::new("io_error", error.to_string()))
    };
    let status = match tokio::time::timeout(timeout, wait).await {
        Ok(outcome) => outcome?,
        Err(_) => {
            kill_process_tree(child_pid);
            return Err(CommandCaptureError::new(
                "timeout",
                format!("command exceeded {}s", timeout.as_secs_f64()),
            ));
        }
    };
    if !status.success() {
        return Err(CommandCaptureError::new(
            "nonzero_exit",
            format!("command exited with {status}"),
        ));
    }
    Ok(String::from_utf8_lossy(&buffer).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn run_command_capture_reads_stdout() {
        let out = run_command_capture("printf 'hello'", std::time::Duration::from_secs(2), 4096)
            .await
            .expect("capture");
        assert_eq!(out, "hello");
    }

    #[tokio::test]
    async fn run_command_capture_rejects_nonzero_exit() {
        let error = run_command_capture("exit 3", std::time::Duration::from_secs(2), 4096)
            .await
            .expect_err("nonzero exit");
        assert_eq!(error.kind, "nonzero_exit");
    }

    #[tokio::test]
    async fn run_command_capture_times_out() {
        let error = run_command_capture("sleep 5", std::time::Duration::from_millis(50), 4096)
            .await
            .expect_err("timeout");
        assert_eq!(error.kind, "timeout");
    }

    #[tokio::test]
    async fn run_command_capture_truncates_stdout() {
        let out = run_command_capture("printf 'abcdefghij'", std::time::Duration::from_secs(2), 4)
            .await
            .expect("capture");
        assert_eq!(out, "abcd");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn run_command_capture_timeout_kills_process_group() {
        let marker = std::env::temp_dir().join(format!(
            "xylitol-host-env-groupkill-{}-{}.tmp",
            std::process::id(),
            line!()
        ));
        let _ = std::fs::remove_file(&marker);
        let script = format!("(sleep 0.6; printf x > '{}') & sleep 30", marker.display());
        let error = run_command_capture(&script, std::time::Duration::from_millis(200), 4096)
            .await
            .expect_err("timeout");
        assert_eq!(error.kind, "timeout");
        tokio::time::sleep(std::time::Duration::from_millis(900)).await;
        assert!(
            !marker.exists(),
            "background child survived the timeout kill: {}",
            marker.display()
        );
        let _ = std::fs::remove_file(&marker);
    }

    #[test]
    fn reserved_credentials_are_declared() {
        assert!(CREDENTIAL_VARS.contains(&"XYLITOL_SERVE_TOKEN"));
        assert!(CREDENTIAL_VARS.contains(&"XYLITOL_AUTH_TOKEN"));
        assert!(!CREDENTIAL_VARS.contains(&"PATH"));
    }
}
