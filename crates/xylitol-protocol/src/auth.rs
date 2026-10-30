use base64::Engine as _;

pub const AUTH_USER: &str = "xylitol";
pub const AUTH_REALM: &str = "xylitol";
pub const AUTHORIZATION_HEADER: &str = "authorization";
pub const AUTH_TOKEN_QUERY: &str = "auth_token";
pub fn basic_authorization(token: &str) -> String {
    let raw = format!("{AUTH_USER}:{token}");
    format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode(raw)
    )
}

pub fn token_from_authorization(value: &str) -> Option<String> {
    let encoded = value
        .strip_prefix("Basic ")
        .or_else(|| value.strip_prefix("basic "))?
        .trim();
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .ok()?;
    let text = String::from_utf8(decoded).ok()?;
    let (user, token) = text.split_once(':')?;
    if user != AUTH_USER || token.is_empty() {
        return None;
    }
    Some(token.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_roundtrip_preserves_token() {
        let header = basic_authorization("s3cret-token");
        assert_eq!(
            token_from_authorization(&header).as_deref(),
            Some("s3cret-token")
        );
    }

    #[test]
    fn rejects_wrong_user_and_empty_token() {
        let wrong_user = format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode("other:token")
        );
        assert!(token_from_authorization(&wrong_user).is_none());
        let empty = format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode("xylitol:")
        );
        assert!(token_from_authorization(&empty).is_none());
    }

    #[test]
    fn rejects_non_basic_and_invalid_base64() {
        assert!(token_from_authorization("Bearer token").is_none());
        assert!(token_from_authorization("Basic !!!").is_none());
        assert!(token_from_authorization("Basic").is_none());
    }
}
