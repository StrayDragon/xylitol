//! Builtin and HuggingFace tokenizer stubs.

use std::path::PathBuf;

use crate::dto::AiBridgeMessage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinTokenizer {
    OpenAiO200k,
    OpenAiCl100k,
    AnthropicClaude,
}

impl BuiltinTokenizer {
    pub fn encode_count(self, text: &str) -> u64 {
        match self {
            Self::OpenAiO200k => tiktoken_rs::o200k_base()
                .map(|enc| enc.encode_with_special_tokens(text).len() as u64)
                .unwrap_or_else(|_| heuristic_count(text)),
            Self::OpenAiCl100k => tiktoken_rs::cl100k_base()
                .map(|enc| enc.encode_with_special_tokens(text).len() as u64)
                .unwrap_or_else(|_| heuristic_count(text)),
            Self::AnthropicClaude => claude_tokenizer::count_tokens(text).unwrap_or(0) as u64,
        }
    }
}

fn heuristic_count(text: &str) -> u64 {
    (text.len() as u64).div_ceil(4)
}

pub fn estimate_messages(messages: &[AiBridgeMessage], tokenizer: BuiltinTokenizer) -> u64 {
    messages
        .iter()
        .map(|msg| {
            let s = serde_json::to_string(msg).unwrap_or_default();
            tokenizer.encode_count(&s)
        })
        .sum()
}

/// HF tokenizer.json cache under `~/.xylitol/tokenizers/` (opt-in download; load if present).
pub struct HfTokenizerCache {
    cache_dir: PathBuf,
}

impl HfTokenizerCache {
    pub fn default_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".xylitol")
            .join("tokenizers")
    }

    pub fn new(cache_dir: Option<PathBuf>) -> Self {
        Self {
            cache_dir: cache_dir.unwrap_or_else(Self::default_dir),
        }
    }

    pub fn cache_path(&self, repo: &str, file: &str) -> PathBuf {
        let safe_repo = repo.replace('/', "__");
        self.cache_dir.join(safe_repo).join(file)
    }

    /// Load tokenizer from cache if present; no network download by default.
    pub fn encode_count_if_cached(&self, repo: &str, file: &str, text: &str) -> Option<u64> {
        let path = self.cache_path(repo, file);
        if !path.exists() {
            return None;
        }
        let tokenizer = tokenizers::Tokenizer::from_file(&path).ok()?;
        let encoding = tokenizer.encode(text, false).ok()?;
        Some(encoding.get_ids().len() as u64)
    }

    /// Opt-in download of `tokenizer.json` into the cache dir.
    ///
    /// MUST NOT be called from default estimate paths; callers pass an explicit
    /// URL (mirrors allowed by the caller).
    pub async fn download_opt_in(
        &self,
        repo: &str,
        file: &str,
        url: &str,
    ) -> Result<std::path::PathBuf, String> {
        let path = self.cache_path(repo, file);
        if path.exists() {
            return Ok(path);
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let bytes = reqwest::get(url)
            .await
            .map_err(|e| e.to_string())?
            .bytes()
            .await
            .map_err(|e| e.to_string())?;
        std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_openai_counts_nonzero() {
        let n = BuiltinTokenizer::OpenAiCl100k.encode_count("hello world");
        assert!(n > 0);
    }

    #[test]
    fn estimate_messages_sums() {
        let msgs = vec![AiBridgeMessage::user("a"), AiBridgeMessage::user("bb")];
        let n = estimate_messages(&msgs, BuiltinTokenizer::OpenAiCl100k);
        assert!(n > 0);
    }

    #[test]
    fn hf_cache_missing_returns_none() {
        let cache = HfTokenizerCache::new(Some(PathBuf::from("/tmp/xylitol-nonexistent-cache")));
        assert!(
            cache
                .encode_count_if_cached("org/model", "tokenizer.json", "hi")
                .is_none()
        );
    }
}
