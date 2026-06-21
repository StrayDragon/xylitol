//! OSC 52 terminal escape sequence support.
//!
//! OSC 52 (Operating System Command 52) allows writing to the system clipboard
//! by emitting a special escape sequence to stdout. This is the universal
//! fallback that works over SSH and in any terminal emulator that supports it.

use std::io::Write;

/// Maximum encoded (base64) payload length for OSC 52.
///
/// Larger payloads can desynchronize terminal rendering and are rejected
/// by some terminal emulators.
pub const MAX_OSC52_ENCODED_LENGTH: usize = 100_000;

/// Check whether the current session is a remote (SSH) session.
pub fn is_remote_session() -> bool {
    std::env::var("SSH_CONNECTION").is_ok()
        || std::env::var("SSH_CLIENT").is_ok()
        || std::env::var("MOSH_CONNECTION").is_ok()
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

/// Emit an OSC 52 escape sequence to set the system clipboard.
///
/// Returns `true` if the sequence was written successfully (within size
/// limits), `false` if the payload exceeds [`MAX_OSC52_ENCODED_LENGTH`].
///
/// # Errors
///
/// Returns an error if writing to stdout fails.
pub fn emit_osc52(text: &str) -> Result<bool, std::io::Error> {
    let encoded = base64_encode(text.as_bytes());
    if encoded.len() > MAX_OSC52_ENCODED_LENGTH {
        return Ok(false);
    }

    let mut stdout = std::io::stdout().lock();
    write!(stdout, "\x1b]52;c;{}\x07", encoded)?;
    stdout.flush()?;
    Ok(true)
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
    fn test_is_remote_session_negative_when_no_env() {
        // SAFETY: test-only env manipulation — single-threaded test context
        let old_ssh = std::env::var("SSH_CONNECTION").ok();
        unsafe { std::env::remove_var("SSH_CONNECTION") };
        let old_client = std::env::var("SSH_CLIENT").ok();
        unsafe { std::env::remove_var("SSH_CLIENT") };

        assert!(!is_remote_session());

        if let Some(v) = old_ssh {
            unsafe { std::env::set_var("SSH_CONNECTION", v) };
        }
        if let Some(v) = old_client {
            unsafe { std::env::set_var("SSH_CLIENT", v) };
        }
    }

    #[test]
    fn test_emit_osc52_returns_ok_for_small_text() {
        // In test context stdout is captured, so this won't actually
        // affect the terminal.
        let result = emit_osc52("small");
        assert!(result.is_ok());
    }
}
