#[derive(Debug, thiserror::Error)]
pub enum AiBridgeError {
    #[error("provider error: {0}")]
    Provider(#[source] anyhow::Error),
    #[error("aborted")]
    Aborted,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_error_display_provider() {
        let err = AiBridgeError::Provider(anyhow::anyhow!("API returned 500"));
        assert_eq!(err.to_string(), "provider error: API returned 500");
    }

    #[test]
    fn bridge_error_display_aborted() {
        let err = AiBridgeError::Aborted;
        assert_eq!(err.to_string(), "aborted");
    }
}
