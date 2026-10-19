pub use xylitol_plugin_sdk::XyError;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_error_display_provider() {
        let err = XyError::Provider(anyhow::anyhow!("API returned 500"));
        assert_eq!(err.to_string(), "provider error: API returned 500");
    }

    #[test]
    fn bridge_error_display_aborted() {
        let err = XyError::Aborted;
        assert_eq!(err.to_string(), "aborted");
    }

    #[test]
    fn bridge_error_display_io() {
        let err = XyError::Provider(anyhow::Error::from(std::io::Error::other(
            "connection reset",
        )));
        assert!(err.to_string().contains("connection reset"), "{}", err);
    }
}
