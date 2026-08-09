//! OSC 52 terminal escape sequence support.
//!
//! OSC 52 (Operating System Command 52) allows writing to the system clipboard
//! by emitting a special escape sequence. This is the universal fallback that
//! works over SSH and in any terminal emulator that supports it.
//!
//! **TUI safety**: prefer [`format_osc52`] and write the sequence on the host
//! thread via `Terminal::write` (outside a differential render batch). Never
//! emit from a `spawn_blocking` worker while the TUI owns stdout — that races
//! CSI 2026 synchronized updates.

use std::io::Write;

/// Maximum encoded (base64) payload length for OSC 52.
///
/// Larger payloads can desynchronize terminal rendering and are rejected
/// by some terminal emulators.
pub const MAX_OSC52_ENCODED_LENGTH: usize = 100_000;

/// Check whether the current session is a remote (SSH) session.
pub fn is_remote_session() -> bool {
    is_remote_session_with(|k| std::env::var(k).ok())
}

/// Injectable remote-session check (reads `SSH_CONNECTION` / `SSH_CLIENT` / `MOSH_CONNECTION`).
pub fn is_remote_session_with(get_env: impl Fn(&str) -> Option<String>) -> bool {
    get_env("SSH_CONNECTION").is_some()
        || get_env("SSH_CLIENT").is_some()
        || get_env("MOSH_CONNECTION").is_some()
}

/// Minimal base64 encoder (RFC 4648) — no external crate needed for OSC 52.
pub(crate) fn base64_encode(input: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

/// Build an OSC 52 sequence without writing anywhere.
///
/// Returns `None` when the base64 payload exceeds `MAX_OSC52_ENCODED_LENGTH` (100_000).
pub fn format_osc52(text: &str) -> Option<String> {
    let encoded = base64_encode(text.as_bytes());
    if encoded.len() > MAX_OSC52_ENCODED_LENGTH {
        return None;
    }
    Some(format!("\x1b]52;c;{encoded}\x07"))
}

/// Write a preformatted OSC 52 sequence to stdout and flush (CLI / non-TUI).
pub fn write_osc52_stdout(sequence: &str) -> Result<(), std::io::Error> {
    let mut stdout = std::io::stdout().lock();
    stdout.write_all(sequence.as_bytes())?;
    stdout.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_encode_basic() {
        assert_eq!(base64_encode(b"hello world"), "aGVsbG8gd29ybGQ=");
    }

    #[test]
    fn test_base64_encode_empty() {
        assert_eq!(base64_encode(b""), "");
    }

    #[test]
    fn test_base64_encode_padding() {
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
    }

    #[test]
    fn test_small_text_fits_in_osc52_limit() {
        let encoded = base64_encode(b"hello world");
        assert!(encoded.len() < MAX_OSC52_ENCODED_LENGTH);
    }

    #[test]
    fn test_large_text_exceeds_osc52_limit() {
        let large = vec![b'a'; MAX_OSC52_ENCODED_LENGTH * 2];
        let encoded = base64_encode(&large);
        assert!(encoded.len() > MAX_OSC52_ENCODED_LENGTH);
    }

    #[test]
    fn format_osc52_matches_emit_shape() {
        let seq = format_osc52("hi").expect("fits");
        assert!(seq.starts_with("\x1b]52;c;"));
        assert!(seq.ends_with('\x07'));
        assert!(seq.contains(&base64_encode(b"hi")));
    }

    #[test]
    fn format_osc52_rejects_oversize() {
        let large = "a".repeat(MAX_OSC52_ENCODED_LENGTH);
        assert!(format_osc52(&large).is_none());
    }

    #[test]
    fn test_is_remote_session_negative_when_no_env() {
        assert!(!is_remote_session_with(|_| None));
    }

    #[test]
    fn test_write_osc52_stdout_ok_for_formatted_sequence() {
        let seq = format_osc52("small").expect("fits");
        // In test context stdout is captured.
        assert!(write_osc52_stdout(&seq).is_ok());
    }
}
