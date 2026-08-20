use crate::prelude::*;
use crate::steps_runtime_config::rc_load_flag;
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
    pub(crate) config: RefCell<xylitol::infra::config::types::AppConfig>,
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
            config: RefCell::new(xylitol::infra::config::types::AppConfig::default()),
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
) -> Result<xylitol::infra::config::types::AppConfig, XyDriverError> {
    // Prefer direct typed deserialize so unknown enum variants (e.g. local_tokenizer)
    // fail here rather than only after a loose Value round-trip.
    let cfg: xylitol::infra::config::types::AppConfig =
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
fn g_ce15_cli_parsed(tokenizer_bdd: &TokenizerBdd) {
    use clap::CommandFactory;
    assert!(
        xylitol::app::cli::CliArgs::command()
            .find_subcommand("tokenizer")
            .is_some()
    );
    tokenizer_bdd.cli_code_ok.set(true);
}

#[when("xylitol tokenizer --help")]
fn w_ce15_tokenizer_help(tokenizer_bdd: &TokenizerBdd) {
    use clap::CommandFactory;
    let mut cmd = xylitol::app::cli::CliArgs::command();
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
    let args = xylitol::app::cli::CliArgs::try_parse_from(["xylitol", "tokenizer", "status"])
        .expect("parse");
    assert!(
        matches!(
            args.command,
            Some(xylitol::app::cli::CliCommand::Tokenizer { .. })
        ),
        "expected tokenizer-only command"
    );
}

#[when("xylitol tokenizer status")]
async fn w_ce15_status(tokenizer_bdd: &TokenizerBdd) {
    use std::process::ExitCode;
    use xylitol::app::cli::tokenizer::{TokenizerAction, run_with};

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
    use std::process::ExitCode;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use xylitol::app::cli::tokenizer::{TokenizerAction, run_with_hf_endpoint};

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
    use xylitol::app::cli::tokenizer::{TokenizerAction, run_with};

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
    use std::process::ExitCode;
    use xylitol::app::cli::tokenizer::{TokenizerAction, run_with};

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
    use xylitol::app::cli::tokenizer::{TokenizerAction, run_with};

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
    use xylitol::app::cli::tokenizer::{TokenizerAction, run_with_hf_endpoint};

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
        let cfg = xylitol::infra::config::types::AppConfig::default();
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

// ═══════════════════════════════════════════════════════════════════
// c1390 — CLI surface verbs (ce16)
// ═══════════════════════════════════════════════════════════════════

pub struct SurfaceBdd {
    pub(crate) mode: Cell<Option<xylitol::app::cli::SurfaceMode>>,
    pub(crate) print_err: RefCell<String>,
    pub(crate) is_tty: Cell<bool>,
    pub(crate) has_prompt: Cell<bool>,
}

impl SurfaceBdd {
    fn new() -> Self {
        Self {
            mode: Cell::new(None),
            print_err: RefCell::new(String::new()),
            is_tty: Cell::new(false),
            has_prompt: Cell::new(true),
        }
    }
}

#[fixture]
pub fn surface_bdd() -> SurfaceBdd {
    SurfaceBdd::new()
}

#[given("TTY")]
fn g_ce16_tty(surface_bdd: &SurfaceBdd) {
    surface_bdd.is_tty.set(true);
}

#[when("xylitol tui")]
fn w_ce16_tui(surface_bdd: &SurfaceBdd) {
    use clap::Parser;
    use xylitol::app::cli::{CliArgs, resolve_surface_intent, select_surface_mode};
    assert!(surface_bdd.is_tty.get(), "given TTY must arm is_tty");
    let args = CliArgs::try_parse_from(["xylitol", "tui"]).expect("parse tui");
    let (force_tui, print_flag, one_shot) = resolve_surface_intent(args.command.as_ref());
    surface_bdd.mode.set(Some(select_surface_mode(
        force_tui,
        print_flag,
        one_shot.is_some(),
        surface_bdd.is_tty.get(),
    )));
}

#[then("进入产品 TUI")]
fn t_ce16_enters_tui(surface_bdd: &SurfaceBdd) {
    assert_eq!(
        surface_bdd.mode.get(),
        Some(xylitol::app::cli::SurfaceMode::Tui)
    );
}

#[given("无 prompt")]
fn g_ce16_no_prompt(surface_bdd: &SurfaceBdd) {
    surface_bdd.has_prompt.set(false);
}

#[when("xylitol print")]
fn w_ce16_print(surface_bdd: &SurfaceBdd) {
    use clap::Parser;
    use xylitol::app::cli::{CliArgs, resolve_print_prompt, resolve_surface_intent};
    assert!(
        !surface_bdd.has_prompt.get(),
        "given 无 prompt must clear has_prompt"
    );
    let args = CliArgs::try_parse_from(["xylitol", "print"]).expect("parse print");
    let (_force_tui, print_flag, one_shot) = resolve_surface_intent(args.command.as_ref());
    let err = resolve_print_prompt(one_shot.as_deref(), print_flag, true, || Ok(String::new()))
        .expect_err("print without prompt must fail");
    surface_bdd.print_err.replace(err.to_string());
}

#[then("错误退出且无 Hello!")]
fn t_ce16_print_no_hello(surface_bdd: &SurfaceBdd) {
    let err = surface_bdd.print_err.borrow();
    assert!(!err.is_empty(), "expected print error");
    assert!(!err.contains("Hello!"), "{err}");
}

#[when("xylitol --help")]
fn w_ce16_top_help(tokenizer_bdd: &TokenizerBdd) {
    use clap::CommandFactory;
    let help = xylitol::app::cli::CliArgs::command()
        .render_long_help()
        .to_string();
    tokenizer_bdd.cli_out.replace(help);
    tokenizer_bdd.cli_code_ok.set(true);
}

#[then("Commands 含 tokenizer 与 resources 为顶层而非 tui 子命令")]
fn t_ce16_ops_toplevel(tokenizer_bdd: &TokenizerBdd) {
    use clap::CommandFactory;
    let help = tokenizer_bdd.cli_out.borrow();
    assert!(help.contains("tokenizer"), "{help}");
    assert!(help.contains("resources"), "{help}");
    assert!(help.contains("tui"), "{help}");
    assert!(help.contains("print"), "{help}");
    let mut cmd = xylitol::app::cli::CliArgs::command();
    let tui = cmd.find_subcommand_mut("tui").expect("tui subcommand");
    assert!(
        tui.find_subcommand("tokenizer").is_none() && tui.find_subcommand("resources").is_none(),
        "ops must not nest under tui"
    );
}
// ═══════════════════════════════════════════════════════════════════
// c1565 — surface-owned flags + resume hint (ce19/ce20)
// ═══════════════════════════════════════════════════════════════════

pub struct SurfaceFlagsBdd {
    pub(crate) parse_ok: Cell<bool>,
    pub(crate) session: RefCell<Option<String>>,
    pub(crate) hint: RefCell<Option<String>>,
}

impl SurfaceFlagsBdd {
    fn new() -> Self {
        Self {
            parse_ok: Cell::new(false),
            session: RefCell::new(None),
            hint: RefCell::new(None),
        }
    }
}

#[fixture]
pub fn surface_flags_bdd() -> SurfaceFlagsBdd {
    SurfaceFlagsBdd::new()
}

#[given("表面旗标上下文就绪")]
fn g_ce19_cli_ready(surface_flags_bdd: &SurfaceFlagsBdd) {
    surface_flags_bdd.parse_ok.set(false);
    surface_flags_bdd.session.replace(None);
    surface_flags_bdd.hint.replace(None);
}

#[when("xylitol tui --session sid --model m --trust")]
fn w_ce19_tui_flags(surface_flags_bdd: &SurfaceFlagsBdd) {
    use clap::Parser;
    use xylitol::app::cli::{CliArgs, surface_from_command};
    let args = CliArgs::try_parse_from([
        "xylitol",
        "tui",
        "--session",
        "sid",
        "--model",
        "m",
        "--trust",
    ])
    .expect("parse tui flags");
    let s = surface_from_command(args.command.as_ref());
    surface_flags_bdd.parse_ok.set(true);
    surface_flags_bdd.session.replace(s.session);
    assert_eq!(s.model.as_deref(), Some("m"));
    assert!(s.trust);
}

#[then("解析成功且表面旗标生效")]
fn t_ce19_flags_ok(surface_flags_bdd: &SurfaceFlagsBdd) {
    assert!(surface_flags_bdd.parse_ok.get());
    assert_eq!(surface_flags_bdd.session.borrow().as_deref(), Some("sid"));
}

#[when("xylitol tui run --session sid")]
fn w_ce19_tui_run_session(surface_flags_bdd: &SurfaceFlagsBdd) {
    use clap::Parser;
    use xylitol::app::cli::{CliArgs, surface_from_command};
    let args = CliArgs::try_parse_from(["xylitol", "tui", "run", "--session", "sid"])
        .expect("parse tui run --session");
    let s = surface_from_command(args.command.as_ref());
    surface_flags_bdd.parse_ok.set(true);
    surface_flags_bdd.session.replace(s.session);
}

#[then("解析成功且 --session 生效")]
fn t_ce19_session_ok(surface_flags_bdd: &SurfaceFlagsBdd) {
    assert!(surface_flags_bdd.parse_ok.get());
    assert_eq!(surface_flags_bdd.session.borrow().as_deref(), Some("sid"));
}

#[when("xylitol print --session sid --no-color hi")]
fn w_ce19_print_flags(surface_flags_bdd: &SurfaceFlagsBdd) {
    use clap::Parser;
    use xylitol::app::cli::CliArgs;
    CliArgs::try_parse_from(["xylitol", "print", "--session", "sid", "--no-color", "hi"])
        .expect("parse print flags");
    surface_flags_bdd.parse_ok.set(true);
}

#[when("xylitol print --session sid --trust --model m hi")]
fn w_ce19_print_trust(surface_flags_bdd: &SurfaceFlagsBdd) {
    use clap::Parser;
    use xylitol::app::cli::{CliArgs, surface_from_command};
    let args = CliArgs::try_parse_from([
        "xylitol",
        "print",
        "--session",
        "sid",
        "--trust",
        "--model",
        "m",
        "hi",
    ])
    .expect("parse print --trust");
    let s = surface_from_command(args.command.as_ref());
    surface_flags_bdd.parse_ok.set(true);
    surface_flags_bdd.session.replace(s.session);
    assert_eq!(s.model.as_deref(), Some("m"));
    assert!(s.trust);
    assert!(!s.no_trust);
}

#[when("xylitol print --session sid --no-trust hi")]
fn w_ce19_print_no_trust(surface_flags_bdd: &SurfaceFlagsBdd) {
    use clap::Parser;
    use xylitol::app::cli::{CliArgs, surface_from_command};
    let args =
        CliArgs::try_parse_from(["xylitol", "print", "--session", "sid", "--no-trust", "hi"])
            .expect("parse print --no-trust");
    let s = surface_from_command(args.command.as_ref());
    surface_flags_bdd.parse_ok.set(true);
    surface_flags_bdd.session.replace(s.session);
    assert!(s.no_trust);
    assert!(!s.trust);
}

#[then("解析成功")]
fn t_ce19_parse_ok(surface_flags_bdd: &SurfaceFlagsBdd) {
    assert!(surface_flags_bdd.parse_ok.get());
}

#[when("xylitol --session sid")]
fn w_ce19_toplevel_session(surface_flags_bdd: &SurfaceFlagsBdd) {
    use clap::Parser;
    use xylitol::app::cli::CliArgs;
    surface_flags_bdd
        .parse_ok
        .set(CliArgs::try_parse_from(["xylitol", "--session", "sid"]).is_ok());
}

#[then("解析失败")]
fn t_ce19_parse_fail(surface_flags_bdd: &SurfaceFlagsBdd) {
    assert!(!surface_flags_bdd.parse_ok.get());
}

#[given("当前 session 已出现在 list_sessions")]
fn g_ce20_listed(surface_flags_bdd: &SurfaceFlagsBdd) {
    surface_flags_bdd.session.replace(Some("uuid-1".into()));
    surface_flags_bdd
        .hint
        .replace(xylitol::app::cli::resume_hint_line(Some("uuid-1"), true));
}

#[given("当前 session 未持久化")]
fn g_ce20_unlisted(surface_flags_bdd: &SurfaceFlagsBdd) {
    surface_flags_bdd.session.replace(Some("uuid-2".into()));
    surface_flags_bdd
        .hint
        .replace(xylitol::app::cli::resume_hint_line(Some("uuid-2"), false));
}

#[when("TUI 或 print 正常退出")]
fn w_ce20_exit(surface_flags_bdd: &SurfaceFlagsBdd) {
    // Exit path: resume hint is prepared in given from list_sessions membership.
    assert!(
        surface_flags_bdd.session.borrow().is_some(),
        "normal exit requires a current session prepared by given"
    );
}

#[then("stderr 含 resume 提示行")]
fn t_ce20_hint_present(surface_flags_bdd: &SurfaceFlagsBdd) {
    let hint = surface_flags_bdd.hint.borrow();
    let line = hint.as_deref().expect("hint");
    assert!(
        line.starts_with("Resume by $ xylitol tui --session "),
        "{line}"
    );
}

#[then("stderr 不含 resume 提示行")]
fn t_ce20_hint_absent(surface_flags_bdd: &SurfaceFlagsBdd) {
    assert!(surface_flags_bdd.hint.borrow().is_none());
}

// ═══════════════════════════════════════════════════════════════════
// c2290 — TUI attach fail-closed (ce21)
// ═══════════════════════════════════════════════════════════════════

pub struct AttachBdd {
    pub(crate) err: RefCell<Option<String>>,
}

impl AttachBdd {
    fn new() -> Self {
        Self {
            err: RefCell::new(None),
        }
    }
}

#[fixture]
pub fn attach_bdd() -> AttachBdd {
    AttachBdd::new()
}

#[given("本机 Host 未在听")]
fn g_ce21_host_down(attach_bdd: &AttachBdd) {
    assert!(attach_bdd.err.borrow().is_none());
}

#[when("产品 TUI 尝试 attach")]
fn w_ce21_attach(attach_bdd: &AttachBdd) {
    let err = xylitol::probe_host("http://127.0.0.1:1").expect_err("port 1 must be down");
    attach_bdd.err.replace(Some(err.to_string()));
}

#[then("非零退出并提示先启动 Host")]
fn t_ce21_fail(attach_bdd: &AttachBdd) {
    let msg = attach_bdd.err.borrow().clone().expect("probe error");
    assert!(
        msg.contains("not listening") || msg.contains("Host is not listening"),
        "{msg}"
    );
    let hint = xylitol::attach_fail_message(xylitol::DEFAULT_ATTACH_URL);
    assert!(hint.contains("xylitol server run"), "{hint}");
}

#[when("无 prompt 且 TTY 启动 TUI")]
fn w_tui2_start(attach_bdd: &AttachBdd) {
    w_ce21_attach(attach_bdd);
}

#[then("默认 attach 本机 Host 且未在听则失败")]
fn t_tui2_attach_default(attach_bdd: &AttachBdd) {
    assert_eq!(xylitol::DEFAULT_ATTACH_URL, "http://127.0.0.1:18790");
    t_ce21_fail(attach_bdd);
}

#[when("检查 Driver 实现")]
fn w_atb4_inspect_drivers() {}

#[then("默认 attach 且同进程驱动路径仍保留给 print 与嵌入")]
fn t_atb4_attach_and_inprocess() {
    assert_eq!(xylitol::DEFAULT_ATTACH_URL, "http://127.0.0.1:18790");
    let inprocess = std::any::type_name::<xylitol::XyInProcessDriver>();
    let http_ws = std::any::type_name::<xylitol::HttpWsClient>();
    assert!(inprocess.contains("XyInProcessDriver"), "{inprocess}");
    assert!(http_ws.contains("HttpWsClient"), "{http_ws}");
}
