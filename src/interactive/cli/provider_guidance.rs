//! Provider-guidance messages — user-facing text for model configuration.
//!
//! Pure CLI-surface presentation (la13): login help, no-model, and
//! no-API-key messages. Not authentication logic — API-key-based only;
//! OAuth login is not supported until after 1.0.0. Lives under interactive/cli/
//! because it is presentation, not agent orchestration.

use crate::core::model::ModelKind;

/// Get help text for provider auth configuration.
///
/// References the `/login` command and the provider/model docs (ux1).
pub fn get_provider_login_help() -> String {
    "Run /login to configure an API key, or set one of these environment variables:\n\
     \x20 OPENAI_API_KEY=sk-...  (for OpenAI-like providers)\n\
     \x20 ANTHROPIC_API_KEY=...   (for Anthropic)\n\
     \x20 Or edit providers.md / models.md (see /login docs).\n\
     \x20 You can also use `xylitol config set api_key <your-key>` to persist a key."
        .to_string()
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
    let env_var = match ModelKind::from_provider_name(provider) {
        Some(ModelKind::OpenAi) => "OPENAI_API_KEY",
        Some(ModelKind::Anthropic) => "ANTHROPIC_API_KEY",
        _ => "<PROVIDER>_API_KEY",
    };
    format!(
        "No API key found for {provider}.\n\n\
         Run /login to configure a key, or set the {env_var} environment variable\n\
         and restart xylitol (see providers.md / models.md).",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login_help() {
        let help = get_provider_login_help();
        assert!(help.contains("API key"));
        assert!(!help.contains("OAuth"));
        assert!(help.contains("/login"));
        assert!(help.contains("providers.md"));
    }

    #[test]
    fn test_no_models_message() {
        let msg = format_no_models_available_message();
        assert!(msg.contains("No models"));
        assert!(msg.contains("/login"));
        assert!(msg.contains("providers.md"));
    }

    #[test]
    fn test_no_model_selected_message() {
        let msg = format_no_model_selected_message();
        assert!(msg.contains("No model selected"));
        assert!(msg.contains("/model"));
        assert!(msg.contains("/login"));
    }

    #[test]
    fn test_no_api_key_message_openai() {
        let msg = format_no_api_key_found_message("openai");
        assert!(msg.contains("openai"));
        assert!(msg.contains("OPENAI_API_KEY"));
        assert!(msg.contains("/login"));
    }

    #[test]
    fn test_no_api_key_message_anthropic() {
        let msg = format_no_api_key_found_message("anthropic");
        assert!(msg.contains("anthropic"));
        assert!(msg.contains("ANTHROPIC_API_KEY"));
        assert!(msg.contains("/login"));
    }

    #[test]
    fn test_no_api_key_message_unknown() {
        let msg = format_no_api_key_found_message("unknown");
        assert!(msg.contains("unknown"));
        assert!(msg.contains("API_KEY"));
    }
}
