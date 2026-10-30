pub const CREDENTIAL_VARS: &[&str] = &["XYLITOL_SERVE_TOKEN", "XYLITOL_AUTH_TOKEN"];

pub fn is_credential_var(name: &str) -> bool {
    CREDENTIAL_VARS.contains(&name)
}

pub fn scrub_tokio_command(command: &mut tokio::process::Command) {
    for var in CREDENTIAL_VARS {
        command.env_remove(*var);
    }
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
