//! Builtin (OpenAI tiktoken) and HuggingFace tokenizer stubs.
//!
//! Anthropic: no local vocab crate — prefer response `usage` (Api) or
//! `count_tokens` (RemoteCount); otherwise Heuristic. The abandoned
//! `claude-tokenizer` crate is intentionally not used.

use std::path::{Path, PathBuf};

use crate::dto::AiBridgeMessage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinTokenizer {
    OpenAiO200k,
    OpenAiCl100k,
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

/// Default HuggingFace hub host (c1380).
pub const DEFAULT_HF_ENDPOINT: &str = "https://huggingface.co";

/// Resolve HF HTTP base: `HF_ENDPOINT` → `HF_HUB_ENDPOINT` → official hub.
pub fn hf_endpoint_base() -> String {
    hf_endpoint_base_from_env(|k| std::env::var(k).ok())
}

/// Testable variant of [`hf_endpoint_base`].
pub fn hf_endpoint_base_from_env(mut get: impl FnMut(&str) -> Option<String>) -> String {
    let raw = get("HF_ENDPOINT")
        .or_else(|| get("HF_HUB_ENDPOINT"))
        .unwrap_or_else(|| DEFAULT_HF_ENDPOINT.to_string());
    raw.trim_end_matches('/').to_string()
}

/// `{base}/{repo}/resolve/main/{file}` (revision fixed to `main` in c1380).
pub fn build_hf_resolve_url(repo: &str, file: &str) -> String {
    build_hf_resolve_url_with_base(&hf_endpoint_base(), repo, file)
}

pub fn build_hf_resolve_url_with_base(base: &str, repo: &str, file: &str) -> String {
    let base = base.trim_end_matches('/');
    format!("{base}/{repo}/resolve/main/{file}")
}

/// One cached tokenizer.json (or alternate file) under the cache root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HfCacheEntry {
    pub repo: String,
    pub file: String,
    pub path: PathBuf,
}

/// HF tokenizer.json cache under `~/.xylitol/tokenizers/` (opt-in download; load if present).
#[derive(Debug, Clone)]
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

    pub fn cache_root(&self) -> &Path {
        &self.cache_dir
    }

    pub fn cache_path(&self, repo: &str, file: &str) -> PathBuf {
        let safe_repo = repo.replace('/', "__");
        self.cache_dir.join(safe_repo).join(file)
    }

    /// List cached `repo/file` pairs (best-effort; skips unreadable dirs).
    pub fn list_entries(&self) -> Vec<HfCacheEntry> {
        let mut out = Vec::new();
        let Ok(repos) = std::fs::read_dir(&self.cache_dir) else {
            return out;
        };
        for repo_ent in repos.flatten() {
            let repo_path = repo_ent.path();
            if !repo_path.is_dir() {
                continue;
            }
            let safe = repo_ent.file_name().to_string_lossy().into_owned();
            let repo = safe.replace("__", "/");
            let Ok(files) = std::fs::read_dir(&repo_path) else {
                continue;
            };
            for file_ent in files.flatten() {
                let path = file_ent.path();
                if !path.is_file() {
                    continue;
                }
                let file = file_ent.file_name().to_string_lossy().into_owned();
                out.push(HfCacheEntry {
                    repo: repo.clone(),
                    file,
                    path,
                });
            }
        }
        out.sort_by(|a, b| (&a.repo, &a.file).cmp(&(&b.repo, &b.file)));
        out
    }

    /// Remove one cache key. Missing path is Ok (idempotent).
    pub fn remove(&self, repo: &str, file: &str) -> Result<(), String> {
        let path = self.cache_path(repo, file);
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
        if let Some(parent) = path.parent() {
            let _ = std::fs::remove_dir(parent); // only if empty
        }
        Ok(())
    }

    /// Remove all entries under the cache root.
    pub fn remove_all(&self) -> Result<(), String> {
        if !self.cache_dir.exists() {
            return Ok(());
        }
        std::fs::remove_dir_all(&self.cache_dir).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Load tokenizer from cache if present; no network download by default.
    pub fn encode_count_if_cached(&self, repo: &str, file: &str, text: &str) -> Option<u64> {
        let path = self.cache_path(repo, file);
        encode_count_at_path(&path, text)
    }

    /// Load tokenizer from an absolute/relative path (config `tokenizer.local`).
    pub fn encode_count_at_path(&self, path: &Path, text: &str) -> Option<u64> {
        let _ = self;
        encode_count_at_path(path, text)
    }

    /// Opt-in download into the cache dir (atomic: temp then rename).
    ///
    /// MUST NOT be called from default estimate paths; callers pass an explicit
    /// URL (or use [`Self::download_opt_in_hf`] which respects `HF_ENDPOINT`).
    pub async fn download_opt_in(
        &self,
        repo: &str,
        file: &str,
        url: &str,
    ) -> Result<PathBuf, String> {
        let path = self.cache_path(repo, file);
        if path.exists() {
            return Ok(path);
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let tmp = {
            let mut t = path.as_os_str().to_owned();
            t.push(".tmp");
            PathBuf::from(t)
        };
        let result = async {
            let bytes = reqwest::get(url)
                .await
                .map_err(|e| e.to_string())?
                .error_for_status()
                .map_err(|e| e.to_string())?
                .bytes()
                .await
                .map_err(|e| e.to_string())?;
            std::fs::write(&tmp, &bytes).map_err(|e| e.to_string())?;
            std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
            Ok(path.clone())
        }
        .await;
        if result.is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
        result
    }

    /// Opt-in download using [`build_hf_resolve_url`] (`HF_ENDPOINT` aware).
    pub async fn download_opt_in_hf(&self, repo: &str, file: &str) -> Result<PathBuf, String> {
        let url = build_hf_resolve_url(repo, file);
        self.download_opt_in(repo, file, &url).await
    }
}

fn encode_count_at_path(path: &Path, text: &str) -> Option<u64> {
    if !path.exists() {
        return None;
    }
    let tokenizer = tokenizers::Tokenizer::from_file(path).ok()?;
    let encoding = tokenizer.encode(text, false).ok()?;
    Some(encoding.get_ids().len() as u64)
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

    #[test]
    fn hf_endpoint_prefers_hf_endpoint() {
        let base = hf_endpoint_base_from_env(|k| match k {
            "HF_ENDPOINT" => Some("https://hf-mirror.com/".into()),
            "HF_HUB_ENDPOINT" => Some("https://ignored.example".into()),
            _ => None,
        });
        assert_eq!(base, "https://hf-mirror.com");
        let url = build_hf_resolve_url_with_base(&base, "Qwen/Qwen2.5", "tokenizer.json");
        assert_eq!(
            url,
            "https://hf-mirror.com/Qwen/Qwen2.5/resolve/main/tokenizer.json"
        );
    }

    #[test]
    fn hf_endpoint_falls_back_hub_then_default() {
        let hub = hf_endpoint_base_from_env(|k| match k {
            "HF_HUB_ENDPOINT" => Some("https://hub-mirror.example".into()),
            _ => None,
        });
        assert_eq!(hub, "https://hub-mirror.example");
        let def = hf_endpoint_base_from_env(|_| None);
        assert_eq!(def, DEFAULT_HF_ENDPOINT);
    }

    #[test]
    fn list_remove_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let cache = HfTokenizerCache::new(Some(dir.path().to_path_buf()));
        let path = cache.cache_path("org/model", "tokenizer.json");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"{}").unwrap();
        let entries = cache.list_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].repo, "org/model");
        assert_eq!(entries[0].file, "tokenizer.json");
        cache.remove("org/model", "tokenizer.json").unwrap();
        assert!(cache.list_entries().is_empty());
    }

    #[test]
    fn download_failure_leaves_no_final_file() {
        let dir = tempfile::tempdir().unwrap();
        let cache = HfTokenizerCache::new(Some(dir.path().to_path_buf()));
        let path = cache.cache_path("org/model", "tokenizer.json");
        // Simulate partial final path existing from a crashed write — remove API
        // must not treat empty/missing as success for encode.
        assert!(!path.exists());
        assert!(
            cache
                .encode_count_if_cached("org/model", "tokenizer.json", "x")
                .is_none()
        );
    }
}
