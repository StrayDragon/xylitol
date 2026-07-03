//! Platform-native clipboard tool wrappers.
//!
//! Each platform has its own clipboard utility (pbcopy, clip, wl-copy, xclip, …).
//! These functions dispatch to the appropriate tool based on the current OS and
//! desktop environment, falling back through the chain on failure.

use std::process::{Command, Stdio};

use super::osc52::{emit_osc52, is_remote_session};

/// Result of a clipboard copy attempt.
#[derive(Debug, Clone, PartialEq)]
pub enum ClipboardResult {
    /// Clipboard was set successfully.
    Copied,
    /// This particular method is not available on the current platform.
    Unsupported,
    /// The tool was found but execution failed.
    Failed(String),
}

/// Copy text to the system clipboard using platform-native tools or OSC 52.
///
/// Strategy (in order):
/// 1. Platform-native tool (pbcopy / clip / wl-copy / xclip / termux-clipboard-set)
/// 2. OSC 52 fallback for remote sessions or when native tools fail
pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    // ── Step 1: Try platform-native tools ──────────────────────────
    let native_result = try_native_copy(text);

    match &native_result {
        ClipboardResult::Copied => return Ok(()),
        ClipboardResult::Unsupported => { /* fall through to OSC 52 */ }
        ClipboardResult::Failed(_e) => { /* fall through to OSC 52 */ }
    }

    // ── Step 2: OSC 52 fallback ────────────────────────────────────
    if is_remote_session() || native_result == ClipboardResult::Unsupported {
        match emit_osc52(text) {
            Ok(true) => return Ok(()),
            Ok(false) => {
                return Err("Clipboard: text exceeds OSC 52 size limit (100KB encoded) \
                     and no native tool available"
                    .to_string());
            }
            Err(e) => {
                return Err(format!("Clipboard: OSC 52 write failed: {e}"));
            }
        }
    }

    Err("Clipboard: no clipboard method available on this platform".to_string())
}

/// Try platform-native clipboard tools.
fn try_native_copy(text: &str) -> ClipboardResult {
    if cfg!(target_os = "macos") {
        copy_macos(text)
    } else if cfg!(target_os = "windows") {
        copy_windows(text)
    } else if cfg!(target_os = "linux") {
        copy_linux(text)
    } else {
        ClipboardResult::Unsupported
    }
}

// ── macOS: pbcopy ────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
fn copy_macos(text: &str) -> ClipboardResult {
    pipe_to_command("pbcopy", &[], text)
}

#[cfg(not(target_os = "macos"))]
fn copy_macos(_text: &str) -> ClipboardResult {
    ClipboardResult::Unsupported
}

// ── Windows: clip ────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn copy_windows(text: &str) -> ClipboardResult {
    pipe_to_command("clip", &[], text)
}

#[cfg(not(target_os = "windows"))]
fn copy_windows(_text: &str) -> ClipboardResult {
    ClipboardResult::Unsupported
}

// ── Linux: wl-copy → xclip → xsel → termux-clipboard-set ────────────

#[cfg(target_os = "linux")]
fn copy_linux(text: &str) -> ClipboardResult {
    // Termux on Android
    if std::env::var("TERMUX_VERSION").is_ok() {
        return pipe_to_command("termux-clipboard-set", &[], text);
    }

    // Wayland
    let has_wayland = std::env::var("WAYLAND_DISPLAY").is_ok()
        || std::env::var("XDG_SESSION_TYPE").as_deref() == Ok("wayland");

    if has_wayland && tool_exists("wl-copy") {
        return spawn_detached("wl-copy", text);
    }

    // X11
    let has_x11 = std::env::var("DISPLAY").is_ok();
    if has_x11 {
        if tool_exists("xclip") {
            return pipe_to_command("xclip", &["-selection", "clipboard"], text);
        }
        if tool_exists("xsel") {
            return pipe_to_command("xsel", &["--clipboard", "--input"], text);
        }
    }

    ClipboardResult::Unsupported
}

#[cfg(not(target_os = "linux"))]
fn copy_linux(_text: &str) -> ClipboardResult {
    ClipboardResult::Unsupported
}

// ── Helpers ──────────────────────────────────────────────────────────

/// Check whether an executable is available on PATH.
fn tool_exists(cmd: &str) -> bool {
    Command::new(cmd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Pipe text to a command via stdin and wait for it to complete.
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
        return ClipboardResult::Failed(e.to_string());
    }

    match child.wait() {
        Ok(status) if status.success() => ClipboardResult::Copied,
        Ok(status) => ClipboardResult::Failed(format!("exit code {:?}", status.code())),
        Err(e) => ClipboardResult::Failed(e.to_string()),
    }
}

/// Spawn a command asynchronously (detached stdin pipe).
///
/// Used for wl-copy which daemonizes and would hang a synchronous wait. We
/// poll for up to ~500ms; if the child is still alive by then, we treat it as
/// daemonized (wl-copy holds the selection in the background by design) and
/// report success without blocking forever.
#[cfg(target_os = "linux")]
fn spawn_detached(cmd: &str, text: &str) -> ClipboardResult {
    use std::io::Write;

    let mut child = match Command::new(cmd)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => return ClipboardResult::Failed(e.to_string()),
    };

    // Write text and close stdin to let wl-copy proceed
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(text.as_bytes());
    }

    // Poll briefly. wl-copy exits promptly on most systems after taking the
    // selection; where it daemonizes (holding the selection), it stays alive
    // until replaced — a blocking wait() would hang forever (the
    // test_copy_to_clipboard_no_panic regression). Give it 500ms, then let go.
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(500);
    loop {
        match child.try_wait() {
            Ok(Some(_status)) => break,
            Ok(None) if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            Ok(None) => {
                // Still running after the deadline — daemonized. Best-effort
                // kill is wrong (would lose the selection); leave it be.
                break;
            }
            Err(_e) => break,
        }
    }

    ClipboardResult::Copied
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clipboard_result_copied() {
        assert_eq!(ClipboardResult::Copied, ClipboardResult::Copied);
    }

    #[test]
    fn test_copy_to_clipboard_no_panic() {
        // In CI/headless environments, this will likely fall through to
        // an error or OSC 52 write (captured by test harness). No panic.
        let _ = copy_to_clipboard("xylitol test");
    }
}
