pub const MAX_OSC52_ENCODED_LENGTH: usize = 100_000;

pub fn is_remote_session_with(get_env: impl Fn(&str) -> Option<String>) -> bool {
    get_env("SSH_CONNECTION").is_some()
        || get_env("SSH_CLIENT").is_some()
        || get_env("MOSH_CONNECTION").is_some()
}

pub(crate) fn base64_encode(input: &[u8]) -> String {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD.encode(input)
}

pub fn format_osc52(text: &str) -> Option<String> {
    let encoded = base64_encode(text.as_bytes());
    if encoded.len() > MAX_OSC52_ENCODED_LENGTH {
        return None;
    }
    Some(format!("\x1b]52;c;{encoded}\x07"))
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
}
