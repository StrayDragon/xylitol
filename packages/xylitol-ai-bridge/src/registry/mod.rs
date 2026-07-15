//! Model id → tokenizer source mapping (builtin OpenAI tiktoken + optional HF).
//!
//! Claude / Anthropic ids intentionally have **no** local builtin: use Api usage
//! or RemoteCount; otherwise accounting falls through to Heuristic.

use crate::tokenize::BuiltinTokenizer;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenizerSource {
    Builtin(BuiltinTokenizer),
    HuggingFace { repo: String, file: String },
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
}
