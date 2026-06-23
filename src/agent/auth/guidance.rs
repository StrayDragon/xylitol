//! Auth guidance — user-facing messages for authentication and model selection.

/// Get help text for provider login, referencing available documentation.
pub fn get_provider_login_help() -> String {
    let docs_path = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".xylitol");
    format!(
        "Use /login to log into a provider via OAuth or API key. See:\n  {}/providers.md\n  {}/models.md",
        docs_path.display(),
        docs_path.display()
    )
}

/// Format a message when no models are available.
pub fn format_no_models_available_message() -> String {
    format!("No models available.\n\n{}", get_provider_login_help())
}

/// Format a message when no model is selected.
pub fn format_no_model_selected_message() -> String {
    format!(
        "No model selected.\n\n{}\n\nThen use /model to select a model.",
        get_provider_login_help()
    )
}

/// Format a message when no API key is found for a provider.
pub fn format_no_api_key_found_message(provider: &str) -> String {
    format!(
        "No API key found for {}.\n\n{}",
        provider,
        get_provider_login_help()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login_help_contains_keywords() {
        let help = get_provider_login_help();
        assert!(help.contains("/login"));
        assert!(help.contains("providers.md"));
        assert!(help.contains("models.md"));
    }

    #[test]
    fn test_no_models_message() {
        let msg = format_no_models_available_message();
        assert!(msg.contains("No models"));
        assert!(msg.contains("/login"));
    }

    #[test]
    fn test_no_model_selected_message() {
        let msg = format_no_model_selected_message();
        assert!(msg.contains("No model selected"));
        assert!(msg.contains("/model"));
    }

    #[test]
    fn test_no_api_key_message() {
        let msg = format_no_api_key_found_message("anthropic");
        assert!(msg.contains("anthropic"));
        assert!(msg.contains("/login"));
    }
}
