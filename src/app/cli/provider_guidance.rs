//! Provider-guidance messages — user-facing text for model configuration.
//!
//! Pure CLI-surface presentation (la13): login help, no-model, and
//! no-API-key messages. Not authentication logic — API-key-based only;
//! OAuth login is not supported until after 1.0.0. Lives under interactive/cli/
//! because it is presentation, not agent orchestration.

use crate::protocol::model::XyModelKind;

/// Get help text for provider auth configuration.
///
/// References the `/login` command and the provider/model docs (ux1).
pub fn get_provider_login_help() -> String {
    "Configure models in config.yaml under models.models, and set each entry's\n\
     \x20 api_key (or {{ secret.NAME }} in secret.env). Examples:\n\
     \x20 DEEPSEEK_API_KEY / OPENCODE_ZEN_API_KEY / a local llama.cpp placeholder.\n\
     \x20 Kind-level OPENAI_API_KEY / ANTHROPIC_API_KEY alone do not invent models.\n\
     \x20 See providers.md / models.md; /login docs are pre-1.0 guidance only."
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
    let env_var = match XyModelKind::from_provider_name(provider) {
        Some(XyModelKind::OpenAi) => "OPENAI_API_KEY",
        Some(XyModelKind::Anthropic) => "ANTHROPIC_API_KEY",
        _ => "<PROVIDER>_API_KEY",
    };
    format!(
        "No API key found for {provider}.\n\n\
         Set models.<alias>.api_key in config.yaml (e.g. {{{{ secret.NAME }}}}),\n\
         or put the secret in secret.env. Kind-level {env_var} is not used as a\n\
         silent fallback for omitted per-model keys (see models.md).",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login_help() {
        let help = get_provider_login_help();
        assert!(help.contains("api_key"));
        assert!(help.contains("models.models"));
        assert!(!help.contains("OAuth"));
        assert!(help.contains("models.md"));
    }

    #[test]
    fn test_no_models_message() {
        let msg = format_no_models_available_message();
        assert!(msg.contains("No models"));
        assert!(msg.contains("models.models"));
        assert!(msg.contains("models.md"));
    }

    #[test]
    fn test_no_model_selected_message() {
        let msg = format_no_model_selected_message();
        assert!(msg.contains("No model selected"));
        assert!(msg.contains("/model"));
        assert!(msg.contains("api_key"));
    }

    #[test]
    fn test_no_api_key_message_openai() {
        let msg = format_no_api_key_found_message("openai");
        assert!(msg.contains("openai"));
        assert!(msg.contains("OPENAI_API_KEY"));
        assert!(msg.contains("api_key"));
        assert!(msg.contains("silent fallback"));
    }

    #[test]
    fn test_no_api_key_message_anthropic() {
        let msg = format_no_api_key_found_message("anthropic");
        assert!(msg.contains("anthropic"));
        assert!(msg.contains("ANTHROPIC_API_KEY"));
        assert!(msg.contains("api_key"));
    }

    #[test]
    fn test_no_api_key_message_unknown() {
        let msg = format_no_api_key_found_message("unknown");
        assert!(msg.contains("unknown"));
        assert!(msg.contains("API_KEY"));
    }
}
