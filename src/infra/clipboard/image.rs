//! Image clipboard reading support.
//!
//! Reads image data from the system clipboard on supported platforms.
//! Uses platform-specific tools (wl-paste, xclip, macOS clipboard, PowerShell).

use std::process::{Command, Stdio};

/// An image read from the system clipboard.
#[derive(Debug, Clone)]
pub struct ClipboardImage {
    /// Raw image bytes (PNG, JPEG, WebP, or GIF).
    pub bytes: Vec<u8>,
    /// MIME type of the image (e.g. "image/png").
    pub mime_type: String,
}

/// Read an image from the system clipboard, if one is available.
///
/// Returns `Ok(None)` if no image is on the clipboard.
/// Returns `Err` if the clipboard tools are unavailable or fail.
pub fn read_clipboard_image() -> Result<Option<ClipboardImage>, String> {
    if cfg!(target_os = "macos") {
        read_macos_clipboard_image()
    } else if cfg!(target_os = "linux") {
        read_linux_clipboard_image()
    } else if cfg!(target_os = "windows") {
        read_windows_clipboard_image()
    } else {
        Err("Clipboard image reading is not supported on this platform".to_string())
    }
}

// ── macOS ────────────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
fn read_macos_clipboard_image() -> Result<Option<ClipboardImage>, String> {
    // Use `osascript` to get the clipboard as TIFF, then convert to PNG
    let script = r#"try
    set theData to the clipboard as «class PNGf»
    return theData
end try"#;

    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .map_err(|e| format!("osascript failed: {e}"))?;

    if !output.status.success() || output.stdout.is_empty() {
        return Ok(None);
    }

    Ok(Some(ClipboardImage {
        bytes: output.stdout,
        mime_type: "image/png".to_string(),
    }))
}

#[cfg(not(target_os = "macos"))]
fn read_macos_clipboard_image() -> Result<Option<ClipboardImage>, String> {
    Err("Not macOS".to_string())
}

// ── Linux (Wayland: wl-paste, X11: xclip) ───────────────────────────

#[cfg(target_os = "linux")]
fn read_linux_clipboard_image() -> Result<Option<ClipboardImage>, String> {
    let has_wayland = std::env::var("WAYLAND_DISPLAY").is_ok()
        || std::env::var("XDG_SESSION_TYPE").as_deref() == Ok("wayland");
    let has_x11 = std::env::var("DISPLAY").is_ok();

    if has_wayland {
        return read_via_wl_paste();
    }

    if has_x11 {
        return read_via_xclip();
    }

    Err("No Wayland or X11 display detected".to_string())
}

#[cfg(not(target_os = "linux"))]
fn read_linux_clipboard_image() -> Result<Option<ClipboardImage>, String> {
    Err("Not Linux".to_string())
}

#[cfg(target_os = "linux")]
fn read_via_wl_paste() -> Result<Option<ClipboardImage>, String> {
    // First check if there's an image by listing MIME types
    let list_output = Command::new("wl-paste")
        .args(["--list-types"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .map_err(|e| format!("wl-paste --list-types failed: {e}"))?;

    let mime_types = String::from_utf8_lossy(&list_output.stdout);
    let mime_type = pick_image_mime_type(&mime_types)?;

    // Read the actual image data
    let output = Command::new("wl-paste")
        .arg("--no-newline")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .map_err(|e| format!("wl-paste failed: {e}"))?;

    if output.stdout.is_empty() {
        return Ok(None);
    }

    Ok(Some(ClipboardImage {
        bytes: output.stdout,
        mime_type,
    }))
}

#[cfg(target_os = "linux")]
fn read_via_xclip() -> Result<Option<ClipboardImage>, String> {
    // xclip -selection clipboard -t image/png -o
    // Try common image types
    let mime_types = ["image/png", "image/jpeg", "image/webp", "image/gif"];

    for mt in &mime_types {
        let output = Command::new("xclip")
            .args(["-selection", "clipboard", "-t", mt, "-o"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .map_err(|e| format!("xclip failed: {e}"))?;

        if output.status.success() && !output.stdout.is_empty() {
            return Ok(Some(ClipboardImage {
                bytes: output.stdout,
                mime_type: mt.to_string(),
            }));
        }
    }

    Ok(None)
}

// ── Windows: PowerShell ──────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn read_windows_clipboard_image() -> Result<Option<ClipboardImage>, String> {
    let script = r#"
Add-Type -AssemblyName System.Windows.Forms
$img = [System.Windows.Forms.Clipboard]::GetImage()
if ($img -ne $null) {
    $ms = New-Object System.IO.MemoryStream
    $img.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    [System.Convert]::ToBase64String($ms.ToArray())
}"#;

    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .map_err(|e| format!("PowerShell failed: {e}"))?;

    if !output.status.success() || output.stdout.is_empty() {
        return Ok(None);
    }

    let b64 = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let bytes = base64_decode(&b64)?;

    Ok(Some(ClipboardImage {
        bytes,
        mime_type: "image/png".to_string(),
    }))
}

#[cfg(not(target_os = "windows"))]
fn read_windows_clipboard_image() -> Result<Option<ClipboardImage>, String> {
    Err("Not Windows".to_string())
}

// ── Helpers ──────────────────────────────────────────────────────────

/// Minimal base64 decoder — mirrors the encoder in osc52.rs.
#[allow(dead_code)]
fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    // Remove whitespace
    let clean: String = input.chars().filter(|c| !c.is_whitespace()).collect();

    // Strip padding
    let padded = clean.trim_end_matches('=');

    // Build decoding table
    let decode = |c: u8| -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some((c - b'A') as u32),
            b'a'..=b'z' => Some((c - b'a' + 26) as u32),
            b'0'..=b'9' => Some((c - b'0' + 52) as u32),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    };

    let bytes = padded.as_bytes();
    let mut result = Vec::with_capacity(bytes.len() / 4 * 3);
    let mut buf = [0u32; 4];

    for chunk in bytes.chunks(4) {
        for (i, &b) in chunk.iter().enumerate() {
            buf[i] = decode(b).ok_or_else(|| format!("Invalid base64 character: {b}"))?;
        }
        let len = chunk.len();
        let triple = match len {
            4 => (buf[0] << 18) | (buf[1] << 12) | (buf[2] << 6) | buf[3],
            3 => (buf[0] << 18) | (buf[1] << 12) | (buf[2] << 6),
            2 => (buf[0] << 18) | (buf[1] << 12),
            _ => return Err("Invalid base64 chunk length".to_string()),
        };
        result.push((triple >> 16) as u8);
        if len > 2 {
            result.push((triple >> 8) as u8);
        }
        if len > 3 {
            result.push(triple as u8);
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_decode_roundtrip() {
        let input = b"hello clipboard";
        let encoded = super::super::osc52::base64_encode(input);
        let decoded = base64_decode(&encoded).unwrap();
        assert_eq!(decoded, input);
    }

    #[test]
    fn test_base64_decode_padding() {
        let decoded = base64_decode("Zg==").unwrap();
        assert_eq!(decoded, b"f");

        let decoded = base64_decode("Zm8=").unwrap();
        assert_eq!(decoded, b"fo");

        let decoded = base64_decode("Zm9v").unwrap();
        assert_eq!(decoded, b"foo");
    }
}
