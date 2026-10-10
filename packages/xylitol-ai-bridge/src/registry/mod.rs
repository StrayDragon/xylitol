//! Model id → tokenizer source + RemoteCount capability mapping.

use std::path::PathBuf;

use crate::tokenize::BuiltinTokenizer;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenizerSource {
    Builtin(BuiltinTokenizer),
    HuggingFace {
        repo: String,
        file: String,
    },
    /// Absolute or relative path to a local `tokenizer.json` (no download).
    Local {
        path: PathBuf,
    },
}

/// Explicit override from product config (`ModelEntry.tokenizer`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenizerOverride {
    Builtin,
    HuggingFace { repo: String, file: String },
    Local { path: PathBuf },
}

/// Which remote token-count API a path can use (c1030 / c1060).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteCountKind {
    /// Anthropic `POST /v1/messages/count_tokens`.
    AnthropicMessages,
    /// OpenAI `POST /v1/responses/input_tokens`.
    OpenAiResponsesInputTokens,
}

/// Builtin mapping for OpenAI-family model ids (Anthropic → `None`).
///
/// Encoding table is tiktoken-rs `get_tokenizer` (aligned with openai/tiktoken
/// 0.14). GPT-5 / 4.1 / 4o / o-series / Codex → o200k; GPT-4 / 3.5 → cl100k;
/// gpt-oss → o200k_harmony. Unknown ids stay unmapped (Heuristic).
pub fn builtin_tokenizer_for(model_id: &str) -> Option<TokenizerSource> {
    BuiltinTokenizer::for_model_id(model_id).map(TokenizerSource::Builtin)
}

/// Resolve tokenizer: optional config override, then tiktoken builtin mapping.
pub fn resolve_tokenizer(model_id: &str) -> Option<TokenizerSource> {
    resolve_tokenizer_with_override(model_id, None)
}

pub fn resolve_tokenizer_with_override(
    model_id: &str,
    over: Option<TokenizerOverride>,
) -> Option<TokenizerSource> {
    if let Some(o) = over {
        return Some(match o {
            TokenizerOverride::Builtin => {
                // Prefer this id's mapping; unmapped aliases get modern o200k.
                builtin_tokenizer_for(model_id)
                    .unwrap_or(TokenizerSource::Builtin(BuiltinTokenizer::FALLBACK))
            }
            TokenizerOverride::HuggingFace { repo, file } => {
                TokenizerSource::HuggingFace { repo, file }
            }
            TokenizerOverride::Local { path } => TokenizerSource::Local { path },
        });
    }
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
    fn override_builtin_unmapped_uses_o200k_fallback() {
        let src = resolve_tokenizer_with_override("qwen-custom", Some(TokenizerOverride::Builtin))
            .unwrap();
        assert_eq!(src, TokenizerSource::Builtin(BuiltinTokenizer::FALLBACK));
    }

    #[test]
    fn override_huggingface_wins() {
        let src = resolve_tokenizer_with_override(
            "unknown-model-xyz",
            Some(TokenizerOverride::HuggingFace {
                repo: "org/m".into(),
                file: "tokenizer.json".into(),
            }),
        )
        .unwrap();
        assert_eq!(
            src,
            TokenizerSource::HuggingFace {
                repo: "org/m".into(),
                file: "tokenizer.json".into(),
            }
        );
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
