//! Tokenizer cache management — `tokenizer status|download|clean` (c1380).
//!
//! Cross-surface ops verb (not under `tui`). Early dispatch: no session/MCP bootstrap.

use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Subcommand;
use xylitol_ai_bridge::registry::{TokenizerSource, resolve_tokenizer_with_override};
use xylitol_ai_bridge::tokenize::{
    HfTokenizerCache, build_hf_resolve_url_with_base, hf_endpoint_base, hf_endpoint_base_from_env,
};

use crate::infra::config::types::AppConfig;

/// Sub-actions for `xylitol tokenizer`.
#[derive(Subcommand, Debug)]
pub enum TokenizerAction {
    /// Show cache root, entries, and optional model mapping status.
    Status {
        /// Model alias / id to resolve.
        #[arg(long)]
        model: Option<String>,
    },
    /// Opt-in download of a HuggingFace tokenizer.json (consent = invoking this).
    Download {
        /// Model alias, or `owner/repo` HF id.
        target: String,
        /// Override file name (default tokenizer.json).
        #[arg(long)]
        file: Option<String>,
        /// Skip TTY confirmation.
        #[arg(long)]
        yes: bool,
    },
    /// Remove cached tokenizer files.
    Clean {
        /// Remove everything under the cache root.
        #[arg(long)]
        all: bool,
        /// Model alias / id.
        #[arg(long)]
        model: Option<String>,
        /// Explicit `owner/repo` (or leftover positional target).
        target: Option<String>,
    },
}

/// Run with default cache dir and loaded config.
pub async fn run(action: TokenizerAction) -> ExitCode {
    let cache = HfTokenizerCache::new(None);
    let config = crate::infra::config::loader::load_app_config(None).unwrap_or_default();
    let (code, out) = run_with(action, &cache, &config, io::stdin().is_terminal()).await;
    print!("{out}");
    code
}

/// Injectable entry for tests (process `HF_ENDPOINT` / default hub).
pub async fn run_with(
    action: TokenizerAction,
    cache: &HfTokenizerCache,
    config: &AppConfig,
    tty: bool,
) -> (ExitCode, String) {
    run_with_hf_endpoint(action, cache, config, tty, None).await
}

/// Like [`run_with`], but `hf_endpoint` overrides process env for URL / `hf_base` lines.
///
/// Pass `Some(base)` for BDD/wiremock without `std::env::set_var("HF_ENDPOINT")`.
pub async fn run_with_hf_endpoint(
    action: TokenizerAction,
    cache: &HfTokenizerCache,
    config: &AppConfig,
    tty: bool,
    hf_endpoint: Option<&str>,
) -> (ExitCode, String) {
    let hf_base = match hf_endpoint {
        Some(ep) => hf_endpoint_base_from_env(|_| Some(ep.to_string())),
        None => hf_endpoint_base(),
    };
    match action {
        TokenizerAction::Status { model } => status(cache, config, model.as_deref(), &hf_base),
        TokenizerAction::Download { target, file, yes } => {
            download(cache, config, &target, file.as_deref(), yes, tty, &hf_base).await
        }
        TokenizerAction::Clean { all, model, target } => {
            clean(cache, config, all, model.as_deref(), target.as_deref())
        }
    }
}

fn status(
    cache: &HfTokenizerCache,
    config: &AppConfig,
    model: Option<&str>,
    hf_base: &str,
) -> (ExitCode, String) {
    let mut out = String::new();
    out.push_str(&format!("cache_root: {}\n", cache.cache_root().display()));
    out.push_str(&format!("hf_base:    {hf_base}\n"));
    let entries = cache.list_entries();
    out.push_str("entries:\n");
    if entries.is_empty() {
        out.push_str("  (none)\n");
    } else {
        for e in &entries {
            out.push_str(&format!(
                "  {} / {} → {}\n",
                e.repo,
                e.file,
                e.path.display()
            ));
        }
    }
    if let Some(id) = model {
        let (label, detail) = model_status_line(cache, config, id);
        out.push_str(&format!("model {id}: {label}"));
        if !detail.is_empty() {
            out.push_str(&format!(" ({detail})"));
        }
        out.push('\n');
    }
    (ExitCode::SUCCESS, out)
}

fn model_status_line(
    cache: &HfTokenizerCache,
    config: &AppConfig,
    model_id: &str,
) -> (String, String) {
    match resolve_for_model(config, model_id) {
        Some(TokenizerSource::Builtin(_)) => ("builtin".into(), "no download needed".into()),
        Some(TokenizerSource::Local { path }) => {
            if path.exists() {
                ("local".into(), path.display().to_string())
            } else {
                (
                    "missing".into(),
                    format!("local path not found: {}", path.display()),
                )
            }
        }
        Some(TokenizerSource::HuggingFace { repo, file }) => {
            let path = cache.cache_path(&repo, &file);
            if path.exists() {
                ("cached".into(), path.display().to_string())
            } else {
                ("missing".into(), format!("{repo}/{file}"))
            }
        }
        None => (
            "unmapped".into(),
            "set models.*.tokenizer or pass owner/repo".into(),
        ),
    }
}

async fn download(
    cache: &HfTokenizerCache,
    config: &AppConfig,
    target: &str,
    file_override: Option<&str>,
    yes: bool,
    tty: bool,
    hf_base: &str,
) -> (ExitCode, String) {
    match resolve_download_target(config, target, file_override) {
        DownloadTarget::Builtin => (
            ExitCode::SUCCESS,
            "builtin tokenizer: no download needed\n".into(),
        ),
        DownloadTarget::Local { path } => (
            ExitCode::SUCCESS,
            format!(
                "local tokenizer configured at {}; no download needed\n",
                path.display()
            ),
        ),
        DownloadTarget::NeedMapping => (
            ExitCode::FAILURE,
            format!(
                "unmapped target `{target}`: set models.*.tokenizer (or tokenizers:) \
                 or pass owner/repo (e.g. Qwen/Qwen2.5-7B-Instruct)\n"
            ),
        ),
        DownloadTarget::Hf { repo, file } => {
            let url = build_hf_resolve_url_with_base(hf_base, &repo, &file);
            let dest = cache.cache_path(&repo, &file);
            let mut summary = format!(
                "Will download:\n  repo: {}\n  file: {}\n  hf_base: {}\n  url: {}\n  dest: {}\n",
                repo,
                file,
                hf_base,
                url,
                dest.display()
            );
            if tty && !yes {
                summary.push_str("Proceed? [y/N] ");
                print!("{summary}");
                let _ = io::stdout().flush();
                let mut line = String::new();
                if io::stdin().read_line(&mut line).is_err() {
                    return (ExitCode::FAILURE, "failed to read confirmation\n".into());
                }
                let ok = matches!(line.trim(), "y" | "Y" | "yes" | "YES");
                if !ok {
                    return (ExitCode::FAILURE, "cancelled\n".into());
                }
                summary.clear();
            } else if !yes && !tty {
                // Non-TTY without --yes: still allow (invocation is consent) but print plan.
            }
            match cache.download_opt_in(&repo, &file, &url).await {
                Ok(path) => (
                    ExitCode::SUCCESS,
                    format!("{summary}downloaded: {}\n", path.display()),
                ),
                Err(e) => (
                    ExitCode::FAILURE,
                    format!("{summary}download failed: {e}\n"),
                ),
            }
        }
    }
}

fn clean(
    cache: &HfTokenizerCache,
    config: &AppConfig,
    all: bool,
    model: Option<&str>,
    target: Option<&str>,
) -> (ExitCode, String) {
    if all {
        return match cache.remove_all() {
            Ok(()) => (
                ExitCode::SUCCESS,
                format!("cleared cache root {}\n", cache.cache_root().display()),
            ),
            Err(e) => (ExitCode::FAILURE, format!("clean --all failed: {e}\n")),
        };
    }
    let key = model.or(target);
    let Some(key) = key else {
        return (
            ExitCode::FAILURE,
            "usage: tokenizer clean --all | --model <id> | <owner/repo>\n".into(),
        );
    };
    let (repo, file) = match resolve_download_target(config, key, None) {
        DownloadTarget::Hf { repo, file } => (repo, file),
        DownloadTarget::Builtin | DownloadTarget::Local { .. } => {
            return (
                ExitCode::SUCCESS,
                "nothing to clean (builtin/local has no HF cache entry)\n".into(),
            );
        }
        DownloadTarget::NeedMapping if key.contains('/') => {
            (key.to_string(), "tokenizer.json".into())
        }
        DownloadTarget::NeedMapping => {
            return (
                ExitCode::FAILURE,
                format!("cannot resolve `{key}` to a cache key\n"),
            );
        }
    };
    let path = cache.cache_path(&repo, &file);
    if !path.exists() {
        return (
            ExitCode::SUCCESS,
            format!("not cached: {repo}/{file} (ok)\n"),
        );
    }
    match cache.remove(&repo, &file) {
        Ok(()) => (ExitCode::SUCCESS, format!("removed {}\n", path.display())),
        Err(e) => (ExitCode::FAILURE, format!("clean failed: {e}\n")),
    }
}

enum DownloadTarget {
    Builtin,
    Local { path: PathBuf },
    Hf { repo: String, file: String },
    NeedMapping,
}

fn resolve_for_model(config: &AppConfig, model_id: &str) -> Option<TokenizerSource> {
    let over = config.tokenizer_override_for(model_id);
    resolve_tokenizer_with_override(model_id, over)
}

fn resolve_download_target(
    config: &AppConfig,
    target: &str,
    file_override: Option<&str>,
) -> DownloadTarget {
    if let Some(src) = resolve_for_model(config, target) {
        return match src {
            TokenizerSource::Builtin(_) => DownloadTarget::Builtin,
            TokenizerSource::Local { path } => DownloadTarget::Local { path },
            TokenizerSource::HuggingFace { repo, file } => DownloadTarget::Hf {
                repo,
                file: file_override.unwrap_or(&file).to_string(),
            },
        };
    }
    if target.contains('/') {
        return DownloadTarget::Hf {
            repo: target.to_string(),
            file: file_override.unwrap_or("tokenizer.json").to_string(),
        };
    }
    DownloadTarget::NeedMapping
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::config::types::{ModelEntry, ModelsConfig, TokenizerEntry};
    use crate::protocol::model::XyModelKind;

    fn cfg_with_shared_tok(alias: &str, tok_name: &str, repo: &str) -> AppConfig {
        let mut models = std::collections::HashMap::new();
        models.insert(
            alias.into(),
            ModelEntry {
                provider: XyModelKind::OpenAi,
                model: "local-quant-id".into(),
                base_url: None,
                api: None,
                compat: None,
                api_key: None,
                fallback: None,
                thinking: true,
                thinking_levels: None,
                thinking_level_map: None,
                context_window: 0,
                tokenizer: Some(tok_name.into()),
            },
        );
        let mut tokenizers = std::collections::HashMap::new();
        tokenizers.insert(
            tok_name.into(),
            TokenizerEntry {
                repo: Some(repo.into()),
                file: "tokenizer.json".into(),
                path: None,
            },
        );
        AppConfig {
            model: ModelsConfig {
                default_model: None,
                models,
            },
            tokenizers,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn status_empty_cache() {
        let dir = tempfile::tempdir().unwrap();
        let cache = HfTokenizerCache::new(Some(dir.path().to_path_buf()));
        let (code, out) = run_with(
            TokenizerAction::Status { model: None },
            &cache,
            &AppConfig::default(),
            false,
        )
        .await;
        assert_eq!(code, ExitCode::SUCCESS);
        assert!(out.contains("cache_root:"));
        assert!(out.contains("(none)"));
    }

    #[tokio::test]
    async fn status_model_unmapped() {
        let dir = tempfile::tempdir().unwrap();
        let cache = HfTokenizerCache::new(Some(dir.path().to_path_buf()));
        let (code, out) = run_with(
            TokenizerAction::Status {
                model: Some("nope".into()),
            },
            &cache,
            &AppConfig::default(),
            false,
        )
        .await;
        assert_eq!(code, ExitCode::SUCCESS);
        assert!(out.contains("unmapped"), "{out}");
    }

    #[tokio::test]
    async fn status_model_mapped_missing() {
        let dir = tempfile::tempdir().unwrap();
        let cache = HfTokenizerCache::new(Some(dir.path().to_path_buf()));
        let cfg = cfg_with_shared_tok("qwen", "qwen36", "org/qwen");
        let (code, out) = run_with(
            TokenizerAction::Status {
                model: Some("qwen".into()),
            },
            &cache,
            &cfg,
            false,
        )
        .await;
        assert_eq!(code, ExitCode::SUCCESS);
        assert!(out.contains("missing"), "{out}");
    }

    #[tokio::test]
    async fn download_unmapped_fails() {
        let dir = tempfile::tempdir().unwrap();
        let cache = HfTokenizerCache::new(Some(dir.path().to_path_buf()));
        let (code, out) = run_with(
            TokenizerAction::Download {
                target: "nope".into(),
                file: None,
                yes: true,
            },
            &cache,
            &AppConfig::default(),
            false,
        )
        .await;
        assert_eq!(code, ExitCode::FAILURE);
        assert!(out.contains("unmapped"), "{out}");
    }

    #[tokio::test]
    async fn clean_missing_is_ok() {
        let dir = tempfile::tempdir().unwrap();
        let cache = HfTokenizerCache::new(Some(dir.path().to_path_buf()));
        let (code, out) = run_with(
            TokenizerAction::Clean {
                all: false,
                model: None,
                target: Some("org/missing".into()),
            },
            &cache,
            &AppConfig::default(),
            false,
        )
        .await;
        assert_eq!(code, ExitCode::SUCCESS);
        assert!(out.contains("not cached"), "{out}");
    }

    #[tokio::test]
    async fn gpt_builtin_download_noop() {
        let dir = tempfile::tempdir().unwrap();
        let cache = HfTokenizerCache::new(Some(dir.path().to_path_buf()));
        let (code, out) = run_with(
            TokenizerAction::Download {
                target: "gpt-4o-mini".into(),
                file: None,
                yes: true,
            },
            &cache,
            &AppConfig::default(),
            false,
        )
        .await;
        assert_eq!(code, ExitCode::SUCCESS);
        assert!(out.contains("builtin"), "{out}");
    }
}
