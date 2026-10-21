use super::error::ClipboardError;
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[derive(Debug, Clone)]
pub struct ClipboardImage {
    pub bytes: Vec<u8>,
    pub mime_type: String,
}

pub fn write_clipboard_image_temp(
    bytes: &[u8],
    mime_type: &str,
) -> Result<PathBuf, ClipboardError> {
    if bytes.is_empty() {
        return Err(ClipboardError::decode("image bytes are empty"));
    }
    let ext = match mime_type {
        "image/jpeg" | "image/jpg" => "jpg",
        "image/webp" => "webp",
        "image/gif" => "gif",
        _ => "png",
    };
    let name = format!("xylitol-paste-{}.{}", uuid::Uuid::new_v4(), ext);
    let path = std::env::temp_dir().join(name);
    std::fs::write(&path, bytes).map_err(|e| ClipboardError::io("write paste image failed", e))?;
    Ok(path)
}

pub fn read_clipboard_image() -> Result<Option<ClipboardImage>, ClipboardError> {
    if cfg!(target_os = "macos") {
        read_macos_clipboard_image()
    } else if cfg!(target_os = "linux") {
        read_linux_clipboard_image()
    } else if cfg!(target_os = "windows") {
        read_windows_clipboard_image()
    } else {
        Err(ClipboardError::unsupported(
            "Clipboard image reading is not supported on this platform",
        ))
    }
}

#[cfg(target_os = "macos")]
fn read_macos_clipboard_image() -> Result<Option<ClipboardImage>, ClipboardError> {
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
        .map_err(|e| ClipboardError::io("osascript failed", e))?;

    if !output.status.success() || output.stdout.is_empty() {
        return Ok(None);
    }

    Ok(Some(ClipboardImage {
        bytes: output.stdout,
        mime_type: "image/png".to_string(),
    }))
}

#[cfg(not(target_os = "macos"))]
fn read_macos_clipboard_image() -> Result<Option<ClipboardImage>, ClipboardError> {
    Err(ClipboardError::unsupported("Not macOS"))
}

#[cfg(target_os = "linux")]
fn read_linux_clipboard_image() -> Result<Option<ClipboardImage>, ClipboardError> {
    let has_wayland = std::env::var("WAYLAND_DISPLAY").is_ok()
        || std::env::var("XDG_SESSION_TYPE").as_deref() == Ok("wayland");
    let has_x11 = std::env::var("DISPLAY").is_ok();

    if has_wayland {
        return read_via_wl_paste();
    }

    if has_x11 {
        return read_via_xclip();
    }

    Err(ClipboardError::unsupported(
        "No Wayland or X11 display detected",
    ))
}

#[cfg(not(target_os = "linux"))]
fn read_linux_clipboard_image() -> Result<Option<ClipboardImage>, ClipboardError> {
    Err(ClipboardError::unsupported("Not Linux"))
}

#[cfg(target_os = "linux")]
fn read_via_wl_paste() -> Result<Option<ClipboardImage>, ClipboardError> {
    let list_output = Command::new("wl-paste")
        .args(["--list-types"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .map_err(|e| ClipboardError::io("wl-paste --list-types failed", e))?;

    if !list_output.status.success() {
        return Ok(None);
    }

    let mime_types = String::from_utf8_lossy(&list_output.stdout);
    let Some(mime_type) = select_preferred_image_mime(&mime_types) else {
        return Ok(None);
    };

    let output = Command::new("wl-paste")
        .args(["--type", &mime_type, "--no-newline"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .map_err(|e| ClipboardError::io("wl-paste failed", e))?;

    if !output.status.success() || output.stdout.is_empty() {
        return Ok(None);
    }

    Ok(Some(ClipboardImage {
        bytes: output.stdout,
        mime_type: base_image_mime(&mime_type).to_string(),
    }))
}

#[cfg(target_os = "linux")]
fn read_via_xclip() -> Result<Option<ClipboardImage>, ClipboardError> {
    let mime_types = ["image/png", "image/jpeg", "image/webp", "image/gif"];

    for mt in &mime_types {
        let output = Command::new("xclip")
            .args(["-selection", "clipboard", "-t", mt, "-o"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .map_err(|e| ClipboardError::io("xclip failed", e))?;

        if output.status.success() && !output.stdout.is_empty() {
            return Ok(Some(ClipboardImage {
                bytes: output.stdout,
                mime_type: mt.to_string(),
            }));
        }
    }

    Ok(None)
}

#[cfg(target_os = "windows")]
fn read_windows_clipboard_image() -> Result<Option<ClipboardImage>, ClipboardError> {
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
        .map_err(|e| ClipboardError::io("PowerShell failed", e))?;

    if !output.status.success() || output.stdout.is_empty() {
        return Ok(None);
    }

    let b64 = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let bytes = decode_base64(&b64)?;

    Ok(Some(ClipboardImage {
        bytes,
        mime_type: "image/png".to_string(),
    }))
}

#[cfg(not(target_os = "windows"))]
fn read_windows_clipboard_image() -> Result<Option<ClipboardImage>, ClipboardError> {
    Err(ClipboardError::unsupported("Not Windows"))
}

#[cfg(any(test, target_os = "linux"))]
const PREFERRED_IMAGE_MIMES: &[&str] = &["image/png", "image/jpeg", "image/webp", "image/gif"];

#[cfg(any(test, target_os = "linux"))]
fn base_image_mime(mime_type: &str) -> &str {
    mime_type.split(';').next().unwrap_or(mime_type).trim()
}

#[cfg(any(test, target_os = "linux"))]
fn select_preferred_image_mime(mime_types: &str) -> Option<String> {
    let entries: Vec<&str> = mime_types
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    for preferred in PREFERRED_IMAGE_MIMES {
        if let Some(raw) = entries
            .iter()
            .find(|t| base_image_mime(t).eq_ignore_ascii_case(preferred))
        {
            return Some((*raw).to_string());
        }
    }

    entries
        .into_iter()
        .find(|t| base_image_mime(t).starts_with("image/"))
        .map(str::to_string)
}

#[cfg(any(target_os = "windows", test))]
fn decode_base64(input: &str) -> Result<Vec<u8>, ClipboardError> {
    let clean: String = input.chars().filter(|c| !c.is_whitespace()).collect();

    let padded = clean.trim_end_matches('=');

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
            buf[i] = decode(b)
                .ok_or_else(|| ClipboardError::decode(format!("Invalid base64 character: {b}")))?;
        }
        let len = chunk.len();
        let triple = match len {
            4 => (buf[0] << 18) | (buf[1] << 12) | (buf[2] << 6) | buf[3],
            3 => (buf[0] << 18) | (buf[1] << 12) | (buf[2] << 6),
            2 => (buf[0] << 18) | (buf[1] << 12),
            _ => return Err(ClipboardError::decode("Invalid base64 chunk length")),
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
    fn test_decode_base64_roundtrip() {
        let input = b"hello clipboard";
        let encoded = super::super::osc52::base64_encode(input);
        let decoded = decode_base64(&encoded).unwrap();
        assert_eq!(decoded, input);
    }

    #[test]
    fn write_clipboard_image_temp_png_suffix() {
        let path = write_clipboard_image_temp(&[1, 2, 3, 4], "image/png").unwrap();
        assert!(path.extension().is_some_and(|e| e == "png"));
        assert!(path.is_file());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn select_mime_prefers_png_over_earlier_bmp_jpeg() {
        let list = "\
image/bmp
image/x-bmp
image/jpeg
image/png
image/webp
";
        assert_eq!(
            select_preferred_image_mime(list).as_deref(),
            Some("image/png")
        );
    }

    #[test]
    fn select_mime_none_when_text_only() {
        assert!(select_preferred_image_mime("text/plain\ntext/html\n").is_none());
    }
}
