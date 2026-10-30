pub const CREDENTIAL_VARS: &[&str] = &["XYLITOL_SERVE_TOKEN", "XYLITOL_AUTH_TOKEN"];

pub fn is_credential_var(name: &str) -> bool {
    CREDENTIAL_VARS.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_reserved_credentials() {
        assert!(is_credential_var("XYLITOL_SERVE_TOKEN"));
        assert!(is_credential_var("XYLITOL_AUTH_TOKEN"));
        assert!(!is_credential_var("PATH"));
    }
}
