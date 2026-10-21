use std::process::{Command, Stdio};

use super::error::ClipboardError;
use super::osc52::{format_osc52, is_remote_session_with};

#[derive(Debug, Clone, PartialEq)]
pub enum ClipboardResult {
    Copied,
    Unsupported,
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardPlan {
    pub native_copied: bool,
    pub want_osc52: bool,
    pub osc52_sequence: Option<String>,
}

impl ClipboardPlan {
    pub fn will_succeed(&self) -> bool {
        self.native_copied || self.osc52_sequence.is_some()
    }

    pub fn failure_message(&self) -> String {
        if self.want_osc52 && self.osc52_sequence.is_none() && !self.native_copied {
            "Clipboard: text exceeds OSC 52 size limit (100KB encoded) \
             and no native tool available"
                .into()
        } else {
            "Clipboard: no clipboard method available on this platform".into()
        }
    }
}

pub fn plan_clipboard_copy(text: &str) -> ClipboardPlan {
    plan_clipboard_copy_with(text, |k| std::env::var(k).ok())
}

pub fn plan_clipboard_copy_with(
    text: &str,
    get_env: impl Fn(&str) -> Option<String>,
) -> ClipboardPlan {
    let path = get_env("PATH");
    let native_copied = matches!(
        try_native_copy_with(text, path.as_deref()),
        ClipboardResult::Copied
    );
    let remote = is_remote_session_with(&get_env);
    let want_osc52 = remote || !native_copied;
    let osc52_sequence = if want_osc52 { format_osc52(text) } else { None };
    ClipboardPlan {
        native_copied,
        want_osc52,
        osc52_sequence,
    }
}

pub async fn plan_clipboard_copy_async(text: String) -> Result<ClipboardPlan, ClipboardError> {
    tokio::task::spawn_blocking(move || plan_clipboard_copy(&text))
        .await
        .map_err(|e| ClipboardError::join(format!("Clipboard: join error: {e}")))
}

fn try_native_copy_with(text: &str, path_value: Option<&str>) -> ClipboardResult {
    if cfg!(target_os = "macos") {
        copy_macos(text, path_value)
    } else if cfg!(target_os = "windows") {
        copy_windows(text, path_value)
    } else if cfg!(target_os = "linux") {
        copy_linux(text, path_value)
    } else {
        ClipboardResult::Unsupported
    }
}

#[cfg(target_os = "macos")]
fn copy_macos(text: &str, path_value: Option<&str>) -> ClipboardResult {
    if !tool_on_path_with("pbcopy", path_value) {
        return ClipboardResult::Unsupported;
    }
    pipe_to_command("pbcopy", &[], text)
}

#[cfg(not(target_os = "macos"))]
fn copy_macos(_text: &str, _path_value: Option<&str>) -> ClipboardResult {
    ClipboardResult::Unsupported
}

#[cfg(target_os = "windows")]
fn copy_windows(text: &str, path_value: Option<&str>) -> ClipboardResult {
    if !tool_on_path_with("clip", path_value) {
        return ClipboardResult::Unsupported;
    }
    pipe_to_command("clip", &[], text)
}

#[cfg(not(target_os = "windows"))]
fn copy_windows(_text: &str, _path_value: Option<&str>) -> ClipboardResult {
    ClipboardResult::Unsupported
}

#[cfg(target_os = "linux")]
fn copy_linux(text: &str, path_value: Option<&str>) -> ClipboardResult {
    if std::env::var("TERMUX_VERSION").is_ok() {
        return pipe_to_command("termux-clipboard-set", &[], text);
    }

    let has_wayland_display = std::env::var_os("WAYLAND_DISPLAY").is_some();
    let is_wayland = has_wayland_display
        || std::env::var("XDG_SESSION_TYPE")
            .map(|v| v.eq_ignore_ascii_case("wayland"))
            .unwrap_or(false);
    let has_x11 = std::env::var_os("DISPLAY").is_some();

    if is_wayland && has_wayland_display && tool_on_path_with("wl-copy", path_value) {
        match spawn_unref_pipe_command("wl-copy", &[], text) {
            ClipboardResult::Copied => return ClipboardResult::Copied,
            other if !has_x11 => return other,
            _ => {}
        }
    }

    if has_x11 {
        if tool_on_path_with("xclip", path_value) {
            return pipe_to_command("xclip", &["-selection", "clipboard"], text);
        }
        if tool_on_path_with("xsel", path_value) {
            return pipe_to_command("xsel", &["--clipboard", "--input"], text);
        }
    }

    ClipboardResult::Unsupported
}

#[cfg(not(target_os = "linux"))]
fn copy_linux(_text: &str, _path_value: Option<&str>) -> ClipboardResult {
    ClipboardResult::Unsupported
}

fn tool_on_path_with(cmd: &str, path_value: Option<&str>) -> bool {
    let Some(path) = path_value else {
        return false;
    };
    for dir in std::env::split_paths(path) {
        let candidate = dir.join(cmd);
        if !candidate.is_file() {
            continue;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = candidate.metadata()
                && meta.permissions().mode() & 0o111 != 0
            {
                return true;
            }
        }
        #[cfg(not(unix))]
        {
            return true;
        }
    }
    false
}

fn pipe_to_command(cmd: &str, args: &[&str], text: &str) -> ClipboardResult {
    use std::io::Write;

    let mut child = match Command::new(cmd)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => return ClipboardResult::Failed(e.to_string()),
    };

    if let Err(e) = child
        .stdin
        .take()
        .expect("stdin was requested")
        .write_all(text.as_bytes())
    {
        let _ = child.kill();
        return ClipboardResult::Failed(e.to_string());
    }

    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(5_000);
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return ClipboardResult::Copied,
            Ok(Some(status)) => {
                return ClipboardResult::Failed(format!("exit code {:?}", status.code()));
            }
            Ok(None) if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return ClipboardResult::Failed(format!("{cmd} timed out after 5s"));
            }
            Err(e) => return ClipboardResult::Failed(e.to_string()),
        }
    }
}

#[cfg(any(test, target_os = "linux"))]
fn spawn_unref_pipe_command(cmd: &str, args: &[&str], text: &str) -> ClipboardResult {
    use std::io::Write;

    let mut child = match Command::new(cmd)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => return ClipboardResult::Failed(e.to_string()),
    };

    if let Some(mut stdin) = child.stdin.take()
        && let Err(e) = stdin.write_all(text.as_bytes())
    {
        let _ = child.kill();
        let _ = child.wait();
        return ClipboardResult::Failed(e.to_string());
    }

    match child.try_wait() {
        Ok(Some(_)) => ClipboardResult::Copied,
        Ok(None) => {
            std::mem::forget(child);
            ClipboardResult::Copied
        }
        Err(e) => ClipboardResult::Failed(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clipboard_result_copied() {
        assert_eq!(ClipboardResult::Copied, ClipboardResult::Copied);
    }

    #[test]
    fn tool_on_path_does_not_execute_clipboard_binaries() {
        let start = std::time::Instant::now();
        let path = std::env::var("PATH").ok();
        let _ = tool_on_path_with("wl-copy", path.as_deref());
        let _ = tool_on_path_with("xclip", path.as_deref());
        let _ = tool_on_path_with("definitely-not-a-real-clipboard-tool-xyz", path.as_deref());
        assert!(
            start.elapsed() < std::time::Duration::from_millis(200),
            "tool_on_path must be a PATH lookup only"
        );
    }

    #[test]
    fn plan_defers_osc52_without_requiring_stdout() {
        let plan = plan_clipboard_copy_with("defer-me", |k| match k {
            "PATH" => Some(String::new()),
            "SSH_CONNECTION" | "SSH_CLIENT" | "MOSH_CONNECTION" => None,
            _ => None,
        });
        assert!(!plan.native_copied);
        assert!(plan.want_osc52);
        let seq = plan
            .osc52_sequence
            .as_deref()
            .expect("small text must format");
        assert!(seq.starts_with("\x1b]52;c;"));
        assert!(plan.will_succeed());
    }

    #[test]
    fn pipe_to_command_times_out_hanging_tool() {
        let dir = tempfile::tempdir().expect("tempdir");
        let hang = dir.path().join("hang-clip");
        std::fs::write(&hang, "#!/bin/sh\nsleep 120\n").expect("write");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&hang).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&hang, perms).unwrap();
        }
        let start = std::time::Instant::now();
        let result = pipe_to_command(hang.to_str().unwrap(), &[], "payload");
        assert!(
            start.elapsed() < std::time::Duration::from_secs(6),
            "pipe_to_command must bound wait at ~5s"
        );
        match result {
            ClipboardResult::Failed(msg) => assert!(msg.contains("timed out"), "{msg}"),
            other => panic!("expected Failed timeout, got {other:?}"),
        }
    }

    #[test]
    fn spawn_unref_returns_without_waiting_for_hanging_child() {
        let dir = tempfile::tempdir().expect("tempdir");
        let hang = dir.path().join("hang-unref");
        std::fs::write(&hang, "#!/bin/sh\ncat >/dev/null\nsleep 120\n").expect("write");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&hang).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&hang, perms).unwrap();
        }
        let start = std::time::Instant::now();
        let result = spawn_unref_pipe_command(hang.to_str().unwrap(), &[], "payload");
        assert!(
            start.elapsed() < std::time::Duration::from_secs(5),
            "unref/forget path must not wait for the hanging child, elapsed={:?}",
            start.elapsed()
        );
        assert_eq!(result, ClipboardResult::Copied);
    }

    #[tokio::test]
    async fn plan_async_completes_within_pipe_timeout() {
        let start = std::time::Instant::now();
        let plan = tokio::task::spawn_blocking(|| {
            plan_clipboard_copy_with("xylitol async clipboard probe", |k| match k {
                "PATH" => Some(String::new()),
                "SSH_CONNECTION" | "SSH_CLIENT" | "MOSH_CONNECTION" => None,
                _ => None,
            })
        })
        .await
        .expect("spawn_blocking join");
        assert!(
            !plan.native_copied,
            "empty PATH must not touch native clipboard"
        );
        assert!(
            start.elapsed() < std::time::Duration::from_secs(6),
            "async plan must not hang past pipe timeout"
        );
    }
}
