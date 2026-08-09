//! Lab (qa-wired): Responses prompt-cache hit → prefix break → miss.
//!
//! Config (programmatic; dedicated file, never the global AppConfig):
//! 1. `XYLITOL_LIVE_PROVIDER_CONFIG` path, else
//! 2. `<global-dir>/dev/live-provider.yaml` (shared via ~/.config/xylitol), else skip
//! 3. Field overrides: `XYLITOL_LIVE_*` / `OPENAI_API_KEY`
//! 4. `XYLITOL_LIVE_PROVIDER=0|1` forces disable/enable
//!
//! Wired into `just qa` via `test-live-provider` (**serial**, `--test-threads=1`).
//! Excluded from nextest parallel matrix (see `.config/nextest.toml`).
//! Naming: unified `lab_` prefix (was `live_responses_prompt_cache`).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use futures::StreamExt;
use serde::Deserialize;
use serde_json::{Value, json};
use xylitol_ai_bridge::dto::{AiBridgeChunk, AiBridgeMessage, PromptCacheRead};
use xylitol_ai_bridge::hooks::{HeaderBag, HttpHooks};
use xylitol_ai_bridge::provider::{AiBridgeLlmAdapter, OpenAiResponsesAdapter};
use xylitol_ai_bridge::{AiBridgeError, AiBridgeGenerateOptions};

#[derive(Debug, Clone, Deserialize)]
struct LiveProviderFile {
    #[serde(default)]
    enabled: bool,
    base_url: String,
    model: String,
    #[serde(default = "default_api_key")]
    api_key: String,
    #[serde(default = "default_max_out")]
    max_output_tokens: u64,
    /// Documented in YAML; recipe enforces serial execution.
    #[serde(default = "default_true")]
    #[allow(dead_code)]
    serial: bool,
}

#[derive(Debug, Clone)]
struct LiveProviderConfig {
    base_url: String,
    model: String,
    api_key: String,
    max_output_tokens: u64,
    source: PathBuf,
}

enum ResolveOutcome {
    Skip(String),
    Run(LiveProviderConfig),
}

fn default_api_key() -> String {
    "sk-local".into()
}
fn default_max_out() -> u64 {
    16
}
fn default_true() -> bool {
    true
}

fn env_truthy(name: &str) -> Option<bool> {
    std::env::var(name).ok().map(|v| {
        matches!(
            v.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

fn load_file(path: &Path) -> Result<LiveProviderFile, String> {
    let raw = std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    yaml_serde::from_str(&raw).map_err(|e| format!("parse {}: {e}", path.display()))
}

fn apply_env_overrides(file: &mut LiveProviderFile) {
    if let Some(on) = env_truthy("XYLITOL_LIVE_PROVIDER") {
        file.enabled = on;
    }
    if let Ok(v) = std::env::var("XYLITOL_LIVE_BASE_URL") {
        if !v.is_empty() {
            file.base_url = v;
        }
    }
    if let Ok(v) = std::env::var("XYLITOL_LIVE_MODEL") {
        if !v.is_empty() {
            file.model = v;
        }
    }
    if let Ok(v) = std::env::var("XYLITOL_LIVE_API_KEY") {
        if !v.is_empty() {
            file.api_key = v;
        }
    } else if let Ok(v) = std::env::var("OPENAI_API_KEY") {
        if !v.is_empty() {
            file.api_key = v;
        }
    }
    if let Ok(v) = std::env::var("XYLITOL_LIVE_MAX_OUTPUT_TOKENS") {
        if let Ok(n) = v.parse() {
            file.max_output_tokens = n;
        }
    }
}

/// Resolve dedicated live-provider config. Never reads the global AppConfig
/// (`~/.config/xylitol/config.yaml`); only the dedicated `dev/live-provider.yaml`.
///
/// `explicit` / `global_dir` are injected so unit tests can cover the full
/// resolve matrix without touching the process environment.
fn resolve_live_provider_from(
    explicit: Option<PathBuf>,
    global_dir: Option<PathBuf>,
) -> ResolveOutcome {
    use xylitol_ai_bridge::config::live_provider_config_path;

    let Some(path) = live_provider_config_path(explicit, global_dir) else {
        return ResolveOutcome::Skip(
            "no XYLITOL_CONFIG_DIR/XDG_CONFIG_HOME/HOME to locate the global config dir".into(),
        );
    };
    if !path.is_file() {
        return ResolveOutcome::Skip(format!(
            "no dedicated config at {} (copy configs/testing/live-provider.example.yaml → <global-dir>/dev/live-provider.yaml, or run `just gen-live-provider-example`)",
            path.display()
        ));
    }

    let mut file = match load_file(&path) {
        Ok(f) => f,
        Err(e) => return ResolveOutcome::Skip(e),
    };
    apply_env_overrides(&mut file);

    if !file.enabled {
        return ResolveOutcome::Skip(format!(
            "enabled=false in {} (set enabled: true or XYLITOL_LIVE_PROVIDER=1)",
            path.display()
        ));
    }
    if file.base_url.trim().is_empty() || file.model.trim().is_empty() {
        return ResolveOutcome::Skip(format!(
            "enabled but base_url/model empty in {}",
            path.display()
        ));
    }

    ResolveOutcome::Run(LiveProviderConfig {
        base_url: file.base_url,
        model: file.model,
        api_key: file.api_key,
        max_output_tokens: file.max_output_tokens,
        source: path,
    })
}

fn resolve_live_provider() -> ResolveOutcome {
    let explicit = std::env::var("XYLITOL_LIVE_PROVIDER_CONFIG")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from);
    let global_dir = xylitol_ai_bridge::config::global_config_dir();
    resolve_live_provider_from(explicit, global_dir)
}

struct CapTokens {
    max_output_tokens: u64,
}

#[async_trait]
impl HttpHooks for CapTokens {
    async fn before_headers(&self, _headers: &mut HeaderBag) -> Result<(), AiBridgeError> {
        Ok(())
    }

    async fn before_request(&self, _model: &str, body: &mut Value) -> Result<(), AiBridgeError> {
        body["max_output_tokens"] = json!(self.max_output_tokens);
        body["store"] = json!(false);
        Ok(())
    }

    async fn after_response(&self, _status: u16, _headers: &HeaderBag) {}
}

fn pad_prefix(tag: &str) -> String {
    format!("{tag} {}", "stable_block ".repeat(80))
}

async fn one_call(
    adapter: &OpenAiResponsesAdapter,
    system: &str,
    user: &str,
) -> xylitol_ai_bridge::dto::AiBridgeUsage {
    let opts = AiBridgeGenerateOptions {
        thinking_level: "off".into(),
        system_prompt: Some(system.to_string()),
        ..Default::default()
    };
    let mut stream = adapter
        .generate(vec![AiBridgeMessage::user(user)], &[], opts)
        .await
        .unwrap_or_else(|e| panic!("generate failed: {e}"));
    let mut usage = None;
    while let Some(item) = stream.next().await {
        match item.unwrap_or_else(|e| panic!("chunk err: {e}")) {
            AiBridgeChunk::Done { usage: u, .. } => usage = u,
            _ => {}
        }
    }
    usage.expect("Done usage missing — provider omitted usage?")
}

/// Counterexample: warm hit then prefix mutation drops cache_read.
///
/// When dedicated config is missing/disabled this test returns early (pass).
/// `just qa` still invokes the binary serially so a local `enabled: true`
/// config is actually verified.
#[tokio::test]
async fn lab_responses_prompt_cache_break_prefix_drops_cache_read() {
    let cfg = match resolve_live_provider() {
        ResolveOutcome::Skip(reason) => {
            eprintln!("live-provider: SKIP — {reason}");
            return;
        }
        ResolveOutcome::Run(c) => c,
    };

    eprintln!(
        "live-provider: RUN source={} model={} base_url={} max_out={}",
        cfg.source.display(),
        cfg.model,
        cfg.base_url,
        cfg.max_output_tokens
    );

    let hooks = Arc::new(CapTokens {
        max_output_tokens: cfg.max_output_tokens,
    });
    let adapter = OpenAiResponsesAdapter::new(
        cfg.api_key.clone(),
        cfg.model.clone(),
        Some(cfg.base_url.clone()),
        Some(hooks),
    );

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let stable = pad_prefix(&format!("XYLITOL_CACHE_IT_{nonce}_A"));
    let broken = pad_prefix(&format!("XYLITOL_CACHE_IT_{nonce}_B"));

    let cold = one_call(&adapter, &stable, "reply with exactly: OK1").await;
    let warm = one_call(&adapter, &stable, "reply with exactly: OK2").await;
    let miss = one_call(&adapter, &broken, "reply with exactly: OK3").await;

    eprintln!(
        "live-provider: cold={:?} warm={:?} miss={:?}",
        cold.prompt_cache_read, warm.prompt_cache_read, miss.prompt_cache_read
    );
    eprintln!(
        "live-provider: cache_read cold={} warm={} miss={} (input≈{})",
        cold.cache_read, warm.cache_read, miss.cache_read, warm.input
    );

    assert!(
        matches!(cold.prompt_cache_read, PromptCacheRead::Tokens(_)),
        "cold must be Tokens(_), got {:?}",
        cold.prompt_cache_read
    );
    assert!(
        matches!(warm.prompt_cache_read, PromptCacheRead::Tokens(_)),
        "warm must be Tokens(_), got {:?}",
        warm.prompt_cache_read
    );
    assert!(
        matches!(miss.prompt_cache_read, PromptCacheRead::Tokens(_)),
        "miss must be Tokens(_), got {:?}",
        miss.prompt_cache_read
    );

    assert!(
        warm.cache_read > cold.cache_read,
        "same-prefix warm must raise cache_read (cold={}, warm={})",
        cold.cache_read,
        warm.cache_read
    );
    assert!(
        warm.cache_read > 0,
        "warm cache_read must be > 0 (got {}); endpoint may lack prompt cache",
        warm.cache_read
    );
    assert!(
        miss.cache_read < warm.cache_read,
        "broken prefix must drop cache_read (warm={}, miss={})",
        warm.cache_read,
        miss.cache_read
    );
    assert_eq!(
        miss.cache_read, 0,
        "expected full cache miss after prefix break (miss={})",
        miss.cache_read
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("xylitol-live-cfg-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_live_provider(dir: &Path, enabled: bool) -> PathBuf {
        let path = dir.join("dev").join("live-provider.yaml");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            format!(
                "enabled: {enabled}\nbase_url: http://127.0.0.1:9/v1\nmodel: test-model\napi_key: sk-local\nmax_output_tokens: 16\nserial: true\n"
            ),
        )
        .unwrap();
        path
    }

    #[test]
    fn resolve_runs_when_global_dev_config_enabled() {
        let dir = temp_dir("run-enabled");
        let path = write_live_provider(&dir, true);
        match resolve_live_provider_from(None, Some(dir.clone())) {
            ResolveOutcome::Run(cfg) => {
                assert_eq!(cfg.source, path);
                assert_eq!(cfg.model, "test-model");
                assert_eq!(cfg.base_url, "http://127.0.0.1:9/v1");
            }
            ResolveOutcome::Skip(reason) => panic!("expected Run, got Skip: {reason}"),
        }
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn resolve_skips_when_global_dev_config_disabled() {
        let dir = temp_dir("run-disabled");
        write_live_provider(&dir, false);
        match resolve_live_provider_from(None, Some(dir.clone())) {
            ResolveOutcome::Skip(reason) => assert!(reason.contains("enabled=false"), "{reason}"),
            ResolveOutcome::Run(cfg) => {
                panic!("expected Skip, got Run at {}", cfg.source.display())
            }
        }
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn resolve_skips_when_config_missing() {
        let dir = temp_dir("missing");
        fs::create_dir_all(dir.join("dev")).unwrap();
        match resolve_live_provider_from(None, Some(dir.clone())) {
            ResolveOutcome::Skip(reason) => {
                assert!(reason.contains("no dedicated config"), "{reason}")
            }
            ResolveOutcome::Run(cfg) => {
                panic!("expected Skip, got Run at {}", cfg.source.display())
            }
        }
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn resolve_skips_without_any_config_dir() {
        match resolve_live_provider_from(None, None) {
            ResolveOutcome::Skip(reason) => {
                assert!(reason.contains("global config dir"), "{reason}")
            }
            ResolveOutcome::Run(cfg) => {
                panic!("expected Skip, got Run at {}", cfg.source.display())
            }
        }
    }

    #[test]
    fn resolve_explicit_env_path_wins() {
        let dir = temp_dir("explicit");
        let path = write_live_provider(&dir, true);
        match resolve_live_provider_from(Some(path.clone()), None) {
            ResolveOutcome::Run(cfg) => assert_eq!(cfg.source, path),
            ResolveOutcome::Skip(reason) => panic!("expected Run, got Skip: {reason}"),
        }
        fs::remove_dir_all(&dir).unwrap();
    }
}
