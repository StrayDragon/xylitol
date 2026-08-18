//! Read UTF-8 text from the system clipboard (c1156 / c8).

use super::error::ClipboardError;
use std::process::{Command, Stdio};

/// Read plain text from the system clipboard, if available.
///
/// Returns `Ok(None)` when there is no text (or only empty).
/// Returns `Err` when clipboard tools are missing or fail hard.
/// Does **not** use OSC 52 (write-only fallback).
pub fn read_clipboard_text() -> Result<Option<String>, ClipboardError> {
    if cfg!(target_os = "macos") {
        read_macos_clipboard_text()
    } else if cfg!(target_os = "linux") {
        read_linux_clipboard_text()
    } else if cfg!(target_os = "windows") {
        read_windows_clipboard_text()
    } else {
        Err(ClipboardError::unsupported(
            "Clipboard text reading is not supported on this platform",
        ))
    }
}

#[cfg(target_os = "macos")]
fn read_macos_clipboard_text() -> Result<Option<String>, ClipboardError> {
    let output = Command::new("pbpaste")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .map_err(|e| ClipboardError::io(format!("pbpaste failed: {e}")))?;
    if !output.status.success() {
        return Ok(None);
    }
    let text = String::from_utf8_lossy(&output.stdout).to_string();
    if text.is_empty() {
        Ok(None)
    } else {
        Ok(Some(text))
    }
}

#[cfg(not(target_os = "macos"))]
fn read_macos_clipboard_text() -> Result<Option<String>, ClipboardError> {
    Err(ClipboardError::unsupported("Not macOS"))
}

#[cfg(target_os = "linux")]
fn read_linux_clipboard_text() -> Result<Option<String>, ClipboardError> {
    let has_wayland = std::env::var("WAYLAND_DISPLAY").is_ok()
        || std::env::var("XDG_SESSION_TYPE").as_deref() == Ok("wayland");
    let has_x11 = std::env::var("DISPLAY").is_ok();

    if has_wayland {
        return read_text_via_wl_paste();
    }
    if has_x11 {
        return read_text_via_xclip();
    }
    Err(ClipboardError::unsupported(
        "No Wayland or X11 display detected",
    ))
}

#[cfg(not(target_os = "linux"))]
fn read_linux_clipboard_text() -> Result<Option<String>, ClipboardError> {
    Err(ClipboardError::unsupported("Not Linux"))
}

#[cfg(target_os = "linux")]
fn read_text_via_wl_paste() -> Result<Option<String>, ClipboardError> {
    let list_output = Command::new("wl-paste")
        .args(["--list-types"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .map_err(|e| ClipboardError::io(format!("wl-paste --list-types failed: {e}")))?;

    if !list_output.status.success() {
        return Ok(None);
    }

    let types = String::from_utf8_lossy(&list_output.stdout);
    let mime = select_text_mime(&types);
    let mut cmd = Command::new("wl-paste");
    cmd.arg("--no-newline");
    if let Some(mt) = mime {
        cmd.args(["--type", mt]);
    }
    let output = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .map_err(|e| ClipboardError::io(format!("wl-paste text failed: {e}")))?;

    if !output.status.success() || output.stdout.is_empty() {
        return Ok(None);
    }
    let text = String::from_utf8_lossy(&output.stdout).to_string();
    if text.is_empty() {
        Ok(None)
    } else {
        Ok(Some(text))
    }
}

#[cfg(target_os = "linux")]
fn read_text_via_xclip() -> Result<Option<String>, ClipboardError> {
    let output = Command::new("xclip")
        .args(["-selection", "clipboard", "-o"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .map_err(|e| ClipboardError::io(format!("xclip failed: {e}")))?;
    if !output.status.success() || output.stdout.is_empty() {
        return Ok(None);
    }
    let text = String::from_utf8_lossy(&output.stdout).to_string();
    if text.is_empty() {
        Ok(None)
    } else {
        Ok(Some(text))
    }
}

/// Prefer `text/plain` (with optional charset) from a MIME list.
fn select_text_mime(mime_types: &str) -> Option<&str> {
    let entries: Vec<&str> = mime_types
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    entries
        .iter()
        .find(|t| {
            let base = t.split(';').next().unwrap_or(t).trim();
            base.eq_ignore_ascii_case("text/plain")
        })
        .copied()
        .or_else(|| {
            entries
                .into_iter()
                .find(|t| t.split(';').next().unwrap_or(t).starts_with("text/"))
        })
}

#[cfg(target_os = "windows")]
fn read_windows_clipboard_text() -> Result<Option<String>, ClipboardError> {
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Get-Clipboard -Raw",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .map_err(|e| ClipboardError::io(format!("PowerShell Get-Clipboard failed: {e}")))?;
    if !output.status.success() {
        return Ok(None);
    }
    let text = String::from_utf8_lossy(&output.stdout).to_string();
    if text.is_empty() {
        Ok(None)
    } else {
        Ok(Some(text))
    }
}

#[cfg(not(target_os = "windows"))]
fn read_windows_clipboard_text() -> Result<Option<String>, ClipboardError> {
    Err(ClipboardError::unsupported("Not Windows"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_text_plain_over_html() {
        let list = "text/html\ntext/plain;charset=utf-8\n";
        assert_eq!(select_text_mime(list), Some("text/plain;charset=utf-8"));
    }

    #[test]
    fn select_text_none_when_image_only() {
        assert!(select_text_mime("image/png\nimage/jpeg\n").is_none());
    }
}
