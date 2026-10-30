pub const CREDENTIAL_VARS: &[&str] = &["XYLITOL_SERVE_TOKEN", "XYLITOL_AUTH_TOKEN"];

pub fn scrub_tokio_command(command: &mut tokio::process::Command) {
    for var in CREDENTIAL_VARS {
        command.env_remove(*var);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserved_credentials_are_declared() {
        assert!(CREDENTIAL_VARS.contains(&"XYLITOL_SERVE_TOKEN"));
        assert!(CREDENTIAL_VARS.contains(&"XYLITOL_AUTH_TOKEN"));
        assert!(!CREDENTIAL_VARS.contains(&"PATH"));
    }
}
