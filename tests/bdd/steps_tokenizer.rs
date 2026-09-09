//! Tokenizer-domain BDD steps shared by three capability families:
//! - `tests/features/cli-entry.feature` CLI tokenizer subcommand (ce15)
//! - `llmanspec/specs/package-ai-bridge-accounting` HfTokenizerCache (paa8/paa9)
//! - `llmanspec/specs/runtime-config` config tokenizer override (rc18/rc19)

use crate::tests::bdd::prelude::*;
use crate::tests::bdd::steps_runtime_config::rc_load_flag;
use rstest::fixture;
use rstest_bdd_macros::{given, then, when};

/// Process-env free HF base for paa9 (avoids `set_var` races under parallel BDD).
enum HfEndpointInject {
    /// Pretend `HF_ENDPOINT=<url>`.
    Endpoint(String),
    /// Pretend neither `HF_ENDPOINT` nor `HF_HUB_ENDPOINT` is set.
    Unset,
}

pub struct TokenizerBdd {
    pub(crate) cache_dir: RefCell<Option<tempfile::TempDir>>,
    pub(crate) cache: RefCell<Option<xylitol_ai_bridge::tokenize::HfTokenizerCache>>,
    pub(crate) config: RefCell<crate::infra::config::types::AppConfig>,
    pub(crate) cli_out: RefCell<String>,
    pub(crate) cli_code_ok: Cell<bool>,
    pub(crate) last_url: RefCell<String>,
    pub(crate) last_list_len: Cell<usize>,
    pub(crate) cfg_ok: Cell<bool>,
    pub(crate) cfg_err: RefCell<String>,
    pub(crate) resolved_hf_repo: RefCell<String>,
    pub(crate) mock_uri: RefCell<String>,
    /// When set, URL assembly / `run_with_hf_endpoint` use this instead of process env.
    hf_inject: RefCell<Option<HfEndpointInject>>,
}

impl TokenizerBdd {
    fn new() -> Self {
        Self {
            cache_dir: RefCell::new(None),
            cache: RefCell::new(None),
            config: RefCell::new(crate::infra::config::types::AppConfig::default()),
            cli_out: RefCell::new(String::new()),
            cli_code_ok: Cell::new(false),
            last_url: RefCell::new(String::new()),
            last_list_len: Cell::new(0),
            cfg_ok: Cell::new(false),
            cfg_err: RefCell::new(String::new()),
            resolved_hf_repo: RefCell::new(String::new()),
            mock_uri: RefCell::new(String::new()),
            hf_inject: RefCell::new(None),
        }
    }

    fn hf_base_for_url(&self) -> String {
        use xylitol_ai_bridge::tokenize::hf_endpoint_base_from_env;
        match self.hf_inject.borrow().as_ref() {
            Some(HfEndpointInject::Endpoint(ep)) => {
                hf_endpoint_base_from_env(|k| (k == "HF_ENDPOINT").then(|| ep.clone()))
            }
            Some(HfEndpointInject::Unset) => hf_endpoint_base_from_env(|_| None),
            None => hf_endpoint_base_from_env(|k| std::env::var(k).ok()),
        }
    }

    /// `Some(base)` for [`run_with_hf_endpoint`] when inject is an endpoint; else process env.
    fn hf_endpoint_override(&self) -> Option<String> {
        match self.hf_inject.borrow().as_ref() {
            Some(HfEndpointInject::Endpoint(ep)) => Some(ep.clone()),
            _ => None,
        }
    }

    fn ensure_cache(&self) -> xylitol_ai_bridge::tokenize::HfTokenizerCache {
        if self.cache.borrow().is_none() {
            let dir = tempfile::tempdir().expect("temp cache");
            let cache =
                xylitol_ai_bridge::tokenize::HfTokenizerCache::new(Some(dir.path().to_path_buf()));
            self.cache_dir.replace(Some(dir));
            self.cache.replace(Some(cache));
        }
        self.cache.borrow().as_ref().unwrap().clone()
    }
}

#[fixture]
pub fn tokenizer_bdd() -> TokenizerBdd {
    TokenizerBdd::new()
}

pub(crate) fn parse_app_config_yaml(
    yaml: &str,
) -> Result<crate::infra::config::types::AppConfig, XyDriverError> {
    // Prefer direct typed deserialize so unknown enum variants (e.g. local_tokenizer)
    // fail here rather than only after a loose Value round-trip.
    let cfg: crate::infra::config::types::AppConfig =
        yaml_serde::from_str(yaml).map_err(|e| format!("yaml: {e}"))?;
    cfg.validate_thinking_levels()?;
    cfg.validate_model_tokenizers()?;
    Ok(cfg)
}

fn minimal_model_yaml(tokenizer_line: &str, with_tokenizers_table: bool) -> String {
    let table = if with_tokenizers_table {
        r#"
tokenizers:
  qwen36:
    repo: Qwen/Qwen3.6-35B-A3B
"#
    } else {
        ""
    };
    format!(
        r#"{table}
models:
  models:
    qwen:
      provider: fake
      model: Qwen3.6-35B-A3B/UD-Q5_K_XL
      {tokenizer_line}
"#
    )
}

#[given("CLI 已解析")]
fn g_ce15_cli_parsed() {
    use clap::CommandFactory;
    assert!(
        crate::app::cli::CliArgs::command()
            .find_subcommand("tokenizer")
            .is_some()
    );
}

#[when("xylitol tokenizer --help")]
fn w_ce15_tokenizer_help(tokenizer_bdd: &TokenizerBdd) {
    use clap::CommandFactory;
    let mut cmd = crate::app::cli::CliArgs::command();
    let help = cmd
        .find_subcommand_mut("tokenizer")
        .expect("tokenizer subcommand")
        .render_long_help()
        .to_string();
    tokenizer_bdd.cli_out.replace(help);
    tokenizer_bdd.cli_code_ok.set(true);
}

#[then("可见 status、download、clean 叶子")]
fn t_ce15_help_leaves(tokenizer_bdd: &TokenizerBdd) {
    let out = tokenizer_bdd.cli_out.borrow();
    assert!(out.contains("status"), "{out}");
    assert!(out.contains("download"), "{out}");
    assert!(out.contains("clean"), "{out}");
}

#[given("缓存目录为空")]
fn g_ce15_empty_cache(tokenizer_bdd: &TokenizerBdd) {
    let cache = tokenizer_bdd.ensure_cache();
    let entries = cache.list_entries();
    assert!(
        entries.is_empty(),
        "fresh cache dir must be empty, got {entries:?}"
    );
}

#[given("仅执行 tokenizer 子命令")]
fn g_ce15_tokenizer_only(tokenizer_bdd: &TokenizerBdd) {
    let _ = tokenizer_bdd.ensure_cache();
    use clap::Parser;
    let args = crate::app::cli::CliArgs::try_parse_from(["xylitol", "tokenizer", "status"])
        .expect("parse");
    assert!(
        matches!(
            args.command,
            Some(crate::app::cli::CliCommand::Tokenizer { .. })
        ),
        "expected tokenizer-only command"
    );
}

#[when("xylitol tokenizer status")]
async fn w_ce15_status(tokenizer_bdd: &TokenizerBdd) {
    use crate::app::cli::tokenizer::{TokenizerAction, run_with};
    use std::process::ExitCode;

    let cache = tokenizer_bdd.ensure_cache();
    let cfg = tokenizer_bdd.config.borrow().clone();
    let (code, out) = run_with(TokenizerAction::Status { model: None }, &cache, &cfg, false).await;
    tokenizer_bdd.cli_out.replace(out);
    tokenizer_bdd.cli_code_ok.set(code == ExitCode::SUCCESS);
}

#[then("报告缓存根且不失败伪装已下载")]
fn t_ce15_status_empty(tokenizer_bdd: &TokenizerBdd) {
    assert!(tokenizer_bdd.cli_code_ok.get());
    let out = tokenizer_bdd.cli_out.borrow();
    assert!(out.contains("cache_root:"), "{out}");
    assert!(out.contains("(none)"), "{out}");
}

#[given("目标映射到 HuggingFace 且本地无缓存")]
fn g_ce15_mapped_missing(tokenizer_bdd: &TokenizerBdd) {
    let yaml = minimal_model_yaml("tokenizer: qwen36", true);
    let cfg = parse_app_config_yaml(&yaml).expect("cfg");
    tokenizer_bdd.config.replace(cfg);
    let _ = tokenizer_bdd.ensure_cache();
}

#[when("xylitol tokenizer download <target> --yes")]
async fn w_ce15_download_yes(tokenizer_bdd: &TokenizerBdd) {
    use crate::app::cli::tokenizer::{TokenizerAction, run_with_hf_endpoint};
    use std::process::ExitCode;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/org/bdd/resolve/main/tokenizer.json"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"{\"version\":\"1.0\"}"))
        .mount(&server)
        .await;

    // Remap to org/bdd so mock path matches.
    let yaml = r#"
tokenizers:
  bddtok:
    repo: org/bdd
models:
  models:
    qwen:
      provider: fake
      model: qwen-x
      tokenizer: bddtok
"#;
    let cfg = parse_app_config_yaml(yaml).expect("cfg");
    tokenizer_bdd.config.replace(cfg);
    tokenizer_bdd.mock_uri.replace(server.uri());
    tokenizer_bdd
        .hf_inject
        .replace(Some(HfEndpointInject::Endpoint(server.uri())));

    let cache = tokenizer_bdd.ensure_cache();
    let cfg = tokenizer_bdd.config.borrow().clone();
    let inject = tokenizer_bdd.hf_endpoint_override();
    let (code, out) = run_with_hf_endpoint(
        TokenizerAction::Download {
            target: "qwen".into(),
            file: None,
            yes: true,
        },
        &cache,
        &cfg,
        false,
        inject.as_deref(),
    )
    .await;
    tokenizer_bdd.cli_out.replace(out);
    tokenizer_bdd.cli_code_ok.set(code == ExitCode::SUCCESS);
}

#[then("词表落入缓存路径且再次 status 可见")]
async fn t_ce15_download_cached(tokenizer_bdd: &TokenizerBdd) {
    use crate::app::cli::tokenizer::{TokenizerAction, run_with};

    assert!(
        tokenizer_bdd.cli_code_ok.get(),
        "download failed: {}",
        tokenizer_bdd.cli_out.borrow()
    );
    let cache = tokenizer_bdd.ensure_cache();
    let cfg = tokenizer_bdd.config.borrow().clone();
    let (_, out) = run_with(TokenizerAction::Status { model: None }, &cache, &cfg, false).await;
    assert!(out.contains("org/bdd") || out.contains("bdd"), "{out}");
    assert!(!out.contains("(none)") || out.contains("entries:"), "{out}");
    let entries = cache.list_entries();
    assert!(
        entries.iter().any(|e| e.repo == "org/bdd"),
        "entries={entries:?}"
    );
}

#[given("缓存中已有条目")]
fn g_ce15_has_entry(tokenizer_bdd: &TokenizerBdd) {
    let cache = tokenizer_bdd.ensure_cache();
    let path = cache.cache_path("org/cached", "tokenizer.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"{}").unwrap();
}

#[when("xylitol tokenizer clean --all")]
async fn w_ce15_clean_all(tokenizer_bdd: &TokenizerBdd) {
    use crate::app::cli::tokenizer::{TokenizerAction, run_with};
    use std::process::ExitCode;

    let cache = tokenizer_bdd.ensure_cache();
    let cfg = tokenizer_bdd.config.borrow().clone();
    let (code, out) = run_with(
        TokenizerAction::Clean {
            all: true,
            model: None,
            target: None,
        },
        &cache,
        &cfg,
        false,
    )
    .await;
    tokenizer_bdd.cli_out.replace(out);
    tokenizer_bdd.cli_code_ok.set(code == ExitCode::SUCCESS);
}

#[then("条目被移除且 status 不再列出")]
async fn t_ce15_clean_gone(tokenizer_bdd: &TokenizerBdd) {
    use crate::app::cli::tokenizer::{TokenizerAction, run_with};

    assert!(tokenizer_bdd.cli_code_ok.get());
    let cache = tokenizer_bdd.ensure_cache();
    assert!(cache.list_entries().is_empty());
    let cfg = tokenizer_bdd.config.borrow().clone();
    let (_, out) = run_with(TokenizerAction::Status { model: None }, &cache, &cfg, false).await;
    assert!(out.contains("(none)"), "{out}");
}

#[then("不经 bootstrap 装配会话或 MCP 即可完成")]
fn t_ce15_no_bootstrap(tokenizer_bdd: &TokenizerBdd) {
    // `run_with` is the early-dispatch leaf used by CLI before bootstrap.
    assert!(tokenizer_bdd.cli_code_ok.get());
    assert!(tokenizer_bdd.cli_out.borrow().contains("cache_root:"));
}

#[given("已设置 HF_ENDPOINT 为镜像基址且目标已映射")]
fn g_ce15_hf_mirror_mapped(tokenizer_bdd: &TokenizerBdd) {
    let yaml = minimal_model_yaml("tokenizer: qwen36", true);
    tokenizer_bdd
        .config
        .replace(parse_app_config_yaml(&yaml).expect("cfg"));
    let _ = tokenizer_bdd.ensure_cache();
    // Fast-fail local "mirror" so summary is asserted without waiting on real HF.
    let mirror = "http://127.0.0.1:9";
    tokenizer_bdd.mock_uri.replace(mirror.into());
    tokenizer_bdd
        .hf_inject
        .replace(Some(HfEndpointInject::Endpoint(mirror.into())));
}

#[when("xylitol tokenizer download <target> 进入确认摘要（或 --yes 的等价日志）")]
async fn w_ce15_download_summary(tokenizer_bdd: &TokenizerBdd) {
    use crate::app::cli::tokenizer::{TokenizerAction, run_with_hf_endpoint};

    let cache = tokenizer_bdd.ensure_cache();
    let cfg = tokenizer_bdd.config.borrow().clone();
    let inject = tokenizer_bdd.hf_endpoint_override();
    // Download may fail (closed port); summary is printed first.
    let (_code, out) = run_with_hf_endpoint(
        TokenizerAction::Download {
            target: "qwen".into(),
            file: None,
            yes: true,
        },
        &cache,
        &cfg,
        false,
        inject.as_deref(),
    )
    .await;
    tokenizer_bdd.cli_out.replace(out);
}

#[then("摘要含该镜像基址与落盘路径")]
fn t_ce15_summary_has_base(tokenizer_bdd: &TokenizerBdd) {
    let out = tokenizer_bdd.cli_out.borrow();
    let base = tokenizer_bdd.mock_uri.borrow();
    assert!(out.contains(&format!("hf_base: {base}")), "{out}");
    assert!(out.contains("dest:"), "{out}");
}

#[given("缓存目录中已有 tokenizer.json 条目")]
fn g_paa8_has_entry(tokenizer_bdd: &TokenizerBdd) {
    g_ce15_has_entry(tokenizer_bdd);
}

#[when("列举并删除该缓存键")]
fn w_paa8_list_remove(tokenizer_bdd: &TokenizerBdd) {
    let cache = tokenizer_bdd.ensure_cache();
    let before = cache.list_entries();
    tokenizer_bdd.last_list_len.set(before.len());
    assert!(before.iter().any(|e| e.repo == "org/cached"));
    cache.remove("org/cached", "tokenizer.json").unwrap();
}

#[then("列举曾包含该条目且删除后不再包含")]
fn t_paa8_removed(tokenizer_bdd: &TokenizerBdd) {
    assert!(tokenizer_bdd.last_list_len.get() >= 1);
    let cache = tokenizer_bdd.ensure_cache();
    assert!(!cache.list_entries().iter().any(|e| e.repo == "org/cached"));
}

#[given("opt-in download 中途失败")]
async fn g_paa8_download_fail(tokenizer_bdd: &TokenizerBdd) {
    let cache = tokenizer_bdd.ensure_cache();
    let err = cache
        .download_opt_in("org/fail", "tokenizer.json", "http://127.0.0.1:9/nope")
        .await;
    assert!(err.is_err(), "expected fail");
}

#[when("再次 encode_count_if_cached")]
fn w_paa8_encode_again(tokenizer_bdd: &TokenizerBdd) {
    let cache = tokenizer_bdd.ensure_cache();
    let n = cache.encode_count_if_cached("org/fail", "tokenizer.json", "x");
    tokenizer_bdd
        .last_list_len
        .set(if n.is_some() { 1 } else { 0 });
}

#[then("不把不完整文件当作可用缓存")]
fn t_paa8_no_partial(tokenizer_bdd: &TokenizerBdd) {
    assert_eq!(tokenizer_bdd.last_list_len.get(), 0);
    let cache = tokenizer_bdd.ensure_cache();
    assert!(!cache.cache_path("org/fail", "tokenizer.json").exists());
}

#[given("环境变量 HF_ENDPOINT 为 https://hf-mirror.com")]
fn g_paa9_mirror(tokenizer_bdd: &TokenizerBdd) {
    tokenizer_bdd
        .hf_inject
        .replace(Some(HfEndpointInject::Endpoint(
            "https://hf-mirror.com".into(),
        )));
}

#[when("拼装某 repo 的 tokenizer.json resolve URL")]
fn w_paa9_build_mirror(tokenizer_bdd: &TokenizerBdd) {
    use xylitol_ai_bridge::tokenize::build_hf_resolve_url_with_base;
    let base = tokenizer_bdd.hf_base_for_url();
    let url = build_hf_resolve_url_with_base(&base, "Qwen/Qwen2.5", "tokenizer.json");
    tokenizer_bdd.last_url.replace(url);
}

#[then("URL 以 https://hf-mirror.com/ 为前缀且含 resolve/main/tokenizer.json")]
fn t_paa9_mirror_url(tokenizer_bdd: &TokenizerBdd) {
    let url = tokenizer_bdd.last_url.borrow();
    assert!(url.starts_with("https://hf-mirror.com/"), "{url}");
    assert!(url.contains("resolve/main/tokenizer.json"), "{url}");
}

#[given("未设置 HF_ENDPOINT 与 HF_HUB_ENDPOINT")]
fn g_paa9_default(tokenizer_bdd: &TokenizerBdd) {
    tokenizer_bdd
        .hf_inject
        .replace(Some(HfEndpointInject::Unset));
}

#[when("拼装 resolve URL")]
fn w_paa9_build_default(tokenizer_bdd: &TokenizerBdd) {
    use xylitol_ai_bridge::tokenize::build_hf_resolve_url_with_base;
    let base = tokenizer_bdd.hf_base_for_url();
    let url = build_hf_resolve_url_with_base(&base, "org/m", "tokenizer.json");
    tokenizer_bdd.last_url.replace(url);
}

#[then("基址为 https://huggingface.co")]
fn t_paa9_default(tokenizer_bdd: &TokenizerBdd) {
    let url = tokenizer_bdd.last_url.borrow();
    assert!(url.starts_with("https://huggingface.co/"), "{url}");
}

#[given("YAML 含 tokenizers.qwen36.repo 且模型条目 tokenizer 为 qwen36")]
fn g_rc18_named(tokenizer_bdd: &TokenizerBdd) {
    let yaml = minimal_model_yaml("tokenizer: qwen36", true);
    match parse_app_config_yaml(&yaml) {
        Ok(cfg) => {
            tokenizer_bdd.config.replace(cfg);
            tokenizer_bdd.cfg_ok.set(true);
        }
        Err(e) => {
            tokenizer_bdd.cfg_ok.set(false);
            tokenizer_bdd.cfg_err.replace(e.to_string());
        }
    }
}

#[when("加载配置")]
fn w_rc18_load(tokenizer_bdd: &TokenizerBdd) {
    if rc_load_flag::LOCAL_ONLY.with(|f| f.get()) {
        let cfg = crate::infra::config::types::AppConfig::default();
        tokenizer_bdd.config.replace(cfg.clone());
        tokenizer_bdd.cfg_ok.set(true);
    } else {
        assert!(
            tokenizer_bdd.cfg_ok.get() || !tokenizer_bdd.cfg_err.borrow().is_empty(),
            "config must be parsed in given"
        );
    }
}

#[then("成功且该模型可解析为 HuggingFace 词表源")]
fn t_rc18_named_ok(tokenizer_bdd: &TokenizerBdd) {
    assert!(
        tokenizer_bdd.cfg_ok.get(),
        "{}",
        tokenizer_bdd.cfg_err.borrow()
    );
    let cfg = tokenizer_bdd.config.borrow();
    let over = cfg.tokenizer_override_for("qwen").expect("override");
    match over {
        xylitol_ai_bridge::registry::TokenizerOverride::HuggingFace { repo, .. } => {
            tokenizer_bdd.resolved_hf_repo.replace(repo);
        }
        other => panic!("expected HF, got {other:?}"),
    }
    assert!(
        tokenizer_bdd
            .resolved_hf_repo
            .borrow()
            .contains("Qwen/Qwen3.6")
    );
}

#[given("模型条目 tokenizer 为 Qwen/Qwen3.6-35B-A3B 字符串")]
fn g_rc18_inline(tokenizer_bdd: &TokenizerBdd) {
    let yaml = minimal_model_yaml("tokenizer: Qwen/Qwen3.6-35B-A3B", false);
    match parse_app_config_yaml(&yaml) {
        Ok(cfg) => {
            tokenizer_bdd.config.replace(cfg);
            tokenizer_bdd.cfg_ok.set(true);
        }
        Err(e) => {
            tokenizer_bdd.cfg_ok.set(false);
            tokenizer_bdd.cfg_err.replace(e.to_string());
        }
    }
}

#[when("解析 tokenizer 引用")]
fn w_rc18_resolve(tokenizer_bdd: &TokenizerBdd) {
    let cfg = tokenizer_bdd.config.borrow();
    let over = cfg.tokenizer_override_for("qwen").expect("override");
    match over {
        xylitol_ai_bridge::registry::TokenizerOverride::HuggingFace { repo, .. } => {
            tokenizer_bdd.resolved_hf_repo.replace(repo);
            tokenizer_bdd.cfg_ok.set(true);
        }
        other => {
            tokenizer_bdd.cfg_ok.set(false);
            tokenizer_bdd.cfg_err.replace(format!("not HF: {other:?}"));
        }
    }
}

#[then("得到 HuggingFace repo 且无需 tokenizers 表项")]
fn t_rc18_inline_ok(tokenizer_bdd: &TokenizerBdd) {
    assert!(tokenizer_bdd.cfg_ok.get());
    assert_eq!(
        tokenizer_bdd.resolved_hf_repo.borrow().as_str(),
        "Qwen/Qwen3.6-35B-A3B"
    );
    assert!(tokenizer_bdd.config.borrow().tokenizers.is_empty());
}

#[given("模型条目 tokenizer 为未知名且非 HF repo/路径/builtin")]
fn g_rc18_bad(tokenizer_bdd: &TokenizerBdd) {
    let yaml = minimal_model_yaml("tokenizer: nope-unknown", false);
    match parse_app_config_yaml(&yaml) {
        Ok(_) => tokenizer_bdd.cfg_ok.set(true),
        Err(e) => {
            tokenizer_bdd.cfg_ok.set(false);
            tokenizer_bdd.cfg_err.replace(e.to_string());
        }
    }
}

#[then("失败")]
fn t_rc18_fail(tokenizer_bdd: &TokenizerBdd) {
    assert!(
        !tokenizer_bdd.cfg_ok.get(),
        "expected validation failure, but cfg_ok=true"
    );
    let err = tokenizer_bdd.cfg_err.borrow();
    assert!(
        err.contains("tokenizer")
            || err.contains("nope")
            || err.contains("thinking_level")
            || err.contains("bogon")
            || err.contains("local_tokenizer"),
        "expected validation error, got: {err}"
    );
}

#[given("YAML 未设 token_estimate.local_tokenizer")]
fn g_rc19_default(tokenizer_bdd: &TokenizerBdd) {
    match parse_app_config_yaml("models: {}\n") {
        Ok(cfg) => {
            tokenizer_bdd.config.replace(cfg);
            tokenizer_bdd.cfg_ok.set(true);
        }
        Err(e) => {
            tokenizer_bdd.cfg_ok.set(false);
            tokenizer_bdd.cfg_err.replace(e.to_string());
        }
    }
}

#[given("YAML 含 token_estimate.local_tokenizer: on")]
fn g_rc19_on(tokenizer_bdd: &TokenizerBdd) {
    match parse_app_config_yaml("models: {}\ntoken_estimate:\n  local_tokenizer: on\n") {
        Ok(cfg) => {
            tokenizer_bdd.config.replace(cfg);
            tokenizer_bdd.cfg_ok.set(true);
            tokenizer_bdd.cfg_err.replace(String::new());
        }
        Err(e) => {
            tokenizer_bdd.cfg_ok.set(false);
            tokenizer_bdd.cfg_err.replace(e.to_string());
        }
    }
}

#[given("YAML 含 token_estimate.local_tokenizer: every_n")]
fn g_rc19_invalid(tokenizer_bdd: &TokenizerBdd) {
    match parse_app_config_yaml("models: {}\ntoken_estimate:\n  local_tokenizer: every_n\n") {
        Ok(cfg) => {
            tokenizer_bdd.config.replace(cfg);
            tokenizer_bdd.cfg_ok.set(true);
            tokenizer_bdd.cfg_err.replace(String::new());
        }
        Err(e) => {
            tokenizer_bdd.cfg_ok.set(false);
            tokenizer_bdd.cfg_err.replace(e.to_string());
        }
    }
}

#[then("local_tokenizer 闸为 off")]
fn t_rc19_off(tokenizer_bdd: &TokenizerBdd) {
    assert!(
        tokenizer_bdd.cfg_ok.get(),
        "{}",
        tokenizer_bdd.cfg_err.borrow()
    );
    assert!(
        !tokenizer_bdd
            .config
            .borrow()
            .token_estimate
            .local_tokenizer
            .is_on()
    );
}

#[then("local_tokenizer 闸为 on")]
fn t_rc19_on(tokenizer_bdd: &TokenizerBdd) {
    assert!(
        tokenizer_bdd.cfg_ok.get(),
        "{}",
        tokenizer_bdd.cfg_err.borrow()
    );
    assert!(
        tokenizer_bdd
            .config
            .borrow()
            .token_estimate
            .local_tokenizer
            .is_on()
    );
}
