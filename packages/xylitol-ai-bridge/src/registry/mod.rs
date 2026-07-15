//! Model id → tokenizer source mapping (builtin + optional HF override stub).

use crate::tokenize::BuiltinTokenizer;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenizerSource {
    Builtin(BuiltinTokenizer),
    HuggingFace { repo: String, file: String },
}

/// Builtin mapping for common OpenAI / Anthropic model ids.
pub fn builtin_tokenizer_for(model_id: &str) -> Option<TokenizerSource> {
    let id = model_id.to_ascii_lowercase();
    if id.starts_with("claude") {
        return Some(TokenizerSource::Builtin(BuiltinTokenizer::AnthropicClaude));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_maps_to_anthropic_builtin() {
        let src = resolve_tokenizer("claude-3-5-sonnet").unwrap();
        assert_eq!(
            src,
            TokenizerSource::Builtin(BuiltinTokenizer::AnthropicClaude)
        );
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
}
