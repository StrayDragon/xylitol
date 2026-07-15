//! Model id → tokenizer source + RemoteCount capability mapping.

use crate::tokenize::BuiltinTokenizer;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenizerSource {
    Builtin(BuiltinTokenizer),
    HuggingFace { repo: String, file: String },
}

/// Which remote token-count API a path can use (c1030 / c1060).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteCountKind {
    /// Anthropic `POST /v1/messages/count_tokens`.
    AnthropicMessages,
    /// OpenAI `POST /v1/responses/input_tokens`.
    OpenAiResponsesInputTokens,
}

/// Builtin mapping for common OpenAI model ids (Anthropic → `None`).
pub fn builtin_tokenizer_for(model_id: &str) -> Option<TokenizerSource> {
    let id = model_id.to_ascii_lowercase();
    if id.starts_with("claude") {
        return None;
    }
    if id.starts_with("gpt-4o") || id.starts_with("o1") || id.starts_with("o3") {
        return Some(TokenizerSource::Builtin(BuiltinTokenizer::OpenAiO200k));
    }
    if id.starts_with("gpt-") {
        return Some(TokenizerSource::Builtin(BuiltinTokenizer::OpenAiCl100k));
    }
    None
}

/// Resolve tokenizer source: user override (future TOML) then builtin map.
pub fn resolve_tokenizer(model_id: &str) -> Option<TokenizerSource> {
    // Stub: user override via `registry.user.toml` — not loaded in scaffold.
    builtin_tokenizer_for(model_id)
}

/// Whether this adapter kind exposes a RemoteCount HTTP API.
///
/// Completions-only paths return `None` (use LocalTokenizer / Api usage instead).
pub fn remote_count_kind_for_adapter(adapter: &str) -> Option<RemoteCountKind> {
    match adapter {
        "anthropic-messages" => Some(RemoteCountKind::AnthropicMessages),
        "openai-responses" => Some(RemoteCountKind::OpenAiResponsesInputTokens),
        "openai-completions" => None,
        _ => None,
    }
}

/// Convenience: Claude models prefer Anthropic count; gpt/o* can use Responses input_tokens
/// when the adapter is Responses (caller still passes adapter string).
pub fn remote_count_kind_for(model_id: &str, adapter: &str) -> Option<RemoteCountKind> {
    let _ = model_id;
    remote_count_kind_for_adapter(adapter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_has_no_local_builtin() {
        assert!(resolve_tokenizer("claude-3-5-sonnet").is_none());
        assert!(resolve_tokenizer("claude-opus-4").is_none());
    }

    #[test]
    fn gpt4o_maps_to_o200k() {
        let src = resolve_tokenizer("gpt-4o-mini").unwrap();
        assert_eq!(src, TokenizerSource::Builtin(BuiltinTokenizer::OpenAiO200k));
    }

    #[test]
    fn unknown_model_returns_none() {
        assert!(resolve_tokenizer("unknown-model-xyz").is_none());
    }

    #[test]
    fn responses_supports_remote_count() {
        assert_eq!(
            remote_count_kind_for_adapter("openai-responses"),
            Some(RemoteCountKind::OpenAiResponsesInputTokens)
        );
        assert_eq!(
            remote_count_kind_for_adapter("anthropic-messages"),
            Some(RemoteCountKind::AnthropicMessages)
        );
        assert_eq!(remote_count_kind_for_adapter("openai-completions"), None);
    }
}
