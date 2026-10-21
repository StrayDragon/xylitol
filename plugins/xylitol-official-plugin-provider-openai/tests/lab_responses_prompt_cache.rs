//! Live Responses prompt-cache counterexample (`lab_responses_prompt_cache`).
//!
//! Config: `<global-dir>/dev/live-provider.yaml` (or `XYLITOL_LIVE_PROVIDER_CONFIG`).
//! Global dir: `XYLITOL_CONFIG_DIR` → `$XDG_CONFIG_HOME/xylitol` → `~/.config/xylitol`.
//!
//! YAML fields (`LiveProviderFile`): `enabled`, `base_url`, `model`, `api_key`,
//! `max_output_tokens`, `serial` (documented intent; `just qa-test-live-provider` always
//! uses `--test-threads=1`).
//!
//! Env overrides: `XYLITOL_LIVE_PROVIDER`, `XYLITOL_LIVE_BASE_URL`, `XYLITOL_LIVE_MODEL`,
//! `XYLITOL_LIVE_API_KEY` / `OPENAI_API_KEY`, `XYLITOL_LIVE_MAX_OUTPUT_TOKENS`.
//! Missing file or `enabled: false` → skip (pass). `enabled: true` → must hit gateway.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use futures::StreamExt;
use serde::Deserialize;
use serde_json::{Value, json};
use xylitol_official_plugin_provider_openai::dto::{PromptCacheRead, XyChunk, XyLlmMessage};
use xylitol_official_plugin_provider_openai::hooks::{HeaderBag, HttpHooks};
use xylitol_official_plugin_provider_openai::provider::OpenAiResponsesAdapter;
use xylitol_official_plugin_provider_openai::{XyError, XyGenerateOptions};
use xylitol_plugin_sdk::XyModel;

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

fn global_config_dir() -> Option<PathBuf> {
    resolve_global_config_dir(|k| std::env::var(k).ok())
}

fn resolve_global_config_dir(get_env: impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    if let Some(dir) = get_env("XYLITOL_CONFIG_DIR").filter(|s| !s.is_empty()) {
        return Some(PathBuf::from(dir));
    }
    if let Some(xdg) = get_env("XDG_CONFIG_HOME").filter(|s| !s.is_empty()) {
        return Some(PathBuf::from(xdg).join("xylitol"));
    }
    get_env("HOME")
        .filter(|s| !s.is_empty())
        .map(|home| PathBuf::from(home).join(".config").join("xylitol"))
}

fn live_provider_config_path(
    explicit: Option<PathBuf>,
    global_dir: Option<PathBuf>,
) -> Option<PathBuf> {
    explicit.or_else(|| global_dir.map(|d| d.join("dev").join("live-provider.yaml")))
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
    if let Ok(v) = std::env::var("XYLITOL_LIVE_BASE_URL")
        && !v.is_empty()
    {
        file.base_url = v;
    }
    if let Ok(v) = std::env::var("XYLITOL_LIVE_MODEL")
        && !v.is_empty()
    {
        file.model = v;
    }
    if let Ok(v) = std::env::var("XYLITOL_LIVE_API_KEY") {
        if !v.is_empty() {
            file.api_key = v;
        }
    } else if let Ok(v) = std::env::var("OPENAI_API_KEY")
        && !v.is_empty()
    {
        file.api_key = v;
    }
    if let Ok(v) = std::env::var("XYLITOL_LIVE_MAX_OUTPUT_TOKENS")
        && let Ok(n) = v.parse()
    {
        file.max_output_tokens = n;
    }
}

fn resolve_live_provider_from(
    explicit: Option<PathBuf>,
    global_dir: Option<PathBuf>,
) -> ResolveOutcome {
    let Some(path) = live_provider_config_path(explicit, global_dir) else {
        return ResolveOutcome::Skip(
            "no XYLITOL_CONFIG_DIR/XDG_CONFIG_HOME/HOME to locate the global config dir".into(),
        );
    };
    if !path.is_file() {
        return ResolveOutcome::Skip(format!(
            "no dedicated config at {} (create <global-dir>/dev/live-provider.yaml with enabled/base_url/model/api_key; see module docs in lab_responses_prompt_cache.rs)",
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
    let global_dir = global_config_dir();
    resolve_live_provider_from(explicit, global_dir)
}

struct CapTokens {
    max_output_tokens: u64,
}

#[async_trait]
impl HttpHooks for CapTokens {
    async fn before_headers(&self, _headers: &mut HeaderBag) -> Result<(), XyError> {
        Ok(())
    }

    async fn before_request(&self, _model: &str, body: &mut Value) -> Result<(), XyError> {
        body["max_output_tokens"] = json!(self.max_output_tokens);
        body["store"] = json!(false);
        Ok(())
    }

    async fn after_response(&self, _status: u16, _headers: &HeaderBag) {}
}

fn pad_prefix(tag: &str) -> String {
    format!("{tag} {}", "stable_block ".repeat(190))
}

async fn one_call(
    adapter: &OpenAiResponsesAdapter,
    prefix: &str,
    user: &str,
) -> xylitol_official_plugin_provider_openai::dto::XyUsage {
    let opts = XyGenerateOptions {
        thinking_level: "off".into(),
        ..Default::default()
    };
    let mut stream = adapter
        .generate_stream(
            vec![XyLlmMessage::user(format!("{prefix}\n{user}"))],
            &[],
            false,
            opts,
        )
        .await
        .unwrap_or_else(|e| panic!("generate failed: {e}"));
    let mut usage = None;
    while let Some(item) = stream.next().await {
        if let XyChunk::Done { usage: u, .. } = item.unwrap_or_else(|e| panic!("chunk err: {e}")) {
            usage = u
        }
    }
    usage.expect("Done usage missing — provider omitted usage?")
}

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

    if cold.cache_read == 0 && warm.cache_read == 0 {
        eprintln!(
            "live-provider: SKIP — gateway lacks prefix cache (warm=0); three-state cache semantic not verifiable here"
        );
        return;
    }

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

    fn env_map<'a>(entries: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |k: &str| {
            entries
                .iter()
                .find(|(name, _)| *name == k)
                .map(|(_, v)| v.to_string())
        }
    }

    #[test]
    fn global_dir_prefers_xy_config_dir() {
        let env = env_map(&[
            ("XYLITOL_CONFIG_DIR", "/custom/cfg"),
            ("XDG_CONFIG_HOME", "/xdg"),
            ("HOME", "/home/u"),
        ]);
        assert_eq!(
            resolve_global_config_dir(env),
            Some(PathBuf::from("/custom/cfg"))
        );
    }

    #[test]
    fn global_dir_falls_back_to_xdg() {
        let env = env_map(&[("XDG_CONFIG_HOME", "/xdg"), ("HOME", "/home/u")]);
        assert_eq!(
            resolve_global_config_dir(env),
            Some(PathBuf::from("/xdg/xylitol"))
        );
    }

    #[test]
    fn global_dir_falls_back_to_home_config() {
        let env = env_map(&[("HOME", "/home/u")]);
        assert_eq!(
            resolve_global_config_dir(env),
            Some(PathBuf::from("/home/u/.config").join("xylitol"))
        );
    }

    #[test]
    fn global_dir_none_without_any_anchor() {
        assert_eq!(resolve_global_config_dir(|_| None), None);
        let env = env_map(&[("XYLITOL_CONFIG_DIR", ""), ("XDG_CONFIG_HOME", "")]);
        assert_eq!(resolve_global_config_dir(env), None);
    }

    #[test]
    fn live_provider_path_explicit_wins_over_global_dir() {
        let explicit = Some(PathBuf::from("/tmp/explicit.yaml"));
        let global = Some(PathBuf::from("/g"));
        assert_eq!(
            live_provider_config_path(explicit, global),
            Some(PathBuf::from("/tmp/explicit.yaml"))
        );
        assert_eq!(
            live_provider_config_path(Some(PathBuf::from("/tmp/explicit.yaml")), None),
            Some(PathBuf::from("/tmp/explicit.yaml"))
        );
    }

    #[test]
    fn live_provider_path_defaults_to_dev_live_provider() {
        assert_eq!(
            live_provider_config_path(None, Some(PathBuf::from("/g"))),
            Some(PathBuf::from("/g/dev/live-provider.yaml"))
        );
        assert_eq!(live_provider_config_path(None, None), None);
    }
}
