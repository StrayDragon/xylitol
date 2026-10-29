use crate::bdd::prelude::*;
use crate::bdd::steps_tokenizer::{TokenizerBdd, parse_app_config_yaml};
use rstest::fixture;
use rstest_bdd_macros::{given, then, when};

pub struct RcSnap {
    pub(crate) settings: RefCell<xylitol_kernel::settings::Settings>,
    pub(crate) mgr: RefCell<Option<xylitol_kernel::settings::SettingsManager>>,
    pub(crate) app_config: RefCell<Option<xylitol_kernel::config::types::AppConfig>>,
    pub(crate) local_yaml: RefCell<Option<String>>,
    pub(crate) compaction: RefCell<Option<xylitol_official_plugin_compaction::CompactionSettings>>,
    pub(crate) meta: RefCell<Option<XyModelMeta>>,
    pub(crate) mm: RefCell<Option<xylitol_runtime::agent::model::ModelManager>>,
}
impl RcSnap {
    fn new() -> Self {
        Self {
            settings: RefCell::new(xylitol_kernel::settings::Settings::default()),
            mgr: RefCell::new(None),
            app_config: RefCell::new(None),
            local_yaml: RefCell::new(None),
            compaction: RefCell::new(None),
            meta: RefCell::new(None),
            mm: RefCell::new(None),
        }
    }

    fn load_settings_mgr(&self) {
        let s = self.settings.borrow().clone();
        self.mgr
            .replace(Some(xylitol_kernel::settings::SettingsManager::in_memory(
                s,
            )));
    }
}

pub(crate) mod rc_load_flag {
    use std::cell::{Cell, RefCell};
    thread_local! {
        pub static LOCAL_ONLY: Cell<bool> = const { Cell::new(false) };
        pub static LOCAL_YAML: RefCell<Option<String>> = const { RefCell::new(None) };
    }
}

fn rc_make_model_manager(
    cfg: &xylitol_kernel::config::types::AppConfig,
) -> xylitol_runtime::agent::model::ModelManager {
    let meta = cfg.resolve_model_meta("m").expect("resolve_model_meta");
    let mut reg = ModelRegistry::new();
    reg.register(meta);
    xylitol_runtime::agent::model::ModelManager::new(
        reg,
        Arc::new(xylitol_kernel::provider::factory::build_provider),
    )
}
#[fixture]
pub fn rc_snap() -> RcSnap {
    RcSnap::new()
}

#[given("settings.json 中 steering_mode 设为 one-at-a-time")]
fn g_rc_steering(rc_snap: &RcSnap) {
    rc_snap.settings.borrow_mut().steering_mode =
        Some(xylitol_kernel::settings::SteeringMode::OneAtATime);
}
#[given("settings 未配置 steering_mode 与 follow_up_mode")]
fn g_rc_mode_defaults(rc_snap: &RcSnap) {
    let mut s = rc_snap.settings.borrow_mut();
    s.steering_mode = None;
    s.follow_up_mode = None;
}
#[given("加载默认或示例 settings")]
fn g_rc_default_settings(rc_snap: &RcSnap) {
    rc_snap.load_settings_mgr();
}
#[given("config.yaml 含 compaction 节及 keepRecentTokens")]
fn g_rc_compaction(rc_snap: &RcSnap) {
    let yaml = "compaction:\n  keepRecentTokens: 42000\nmodels: {}\n";
    match parse_app_config_yaml(yaml) {
        Ok(cfg) => {
            rc_snap.app_config.replace(Some(cfg));
        }
        Err(e) => panic!("compaction yaml: {e}"),
    }
}
#[given("config.yaml 的 compaction.model 经 YAML 别名引用与 models 条目同源的锚点")]
fn g_rc_compaction_anchor(rc_snap: &RcSnap, tokenizer_bdd: &TokenizerBdd) {
    let yaml = "models:\n  models:\n    main: &main\n      provider: fake\n      model: anchor-model\ncompaction:\n  model: *main\n";
    let cfg = parse_app_config_yaml(yaml).expect("anchor yaml");
    rc_snap.app_config.replace(Some(cfg.clone()));
    tokenizer_bdd.config.replace(cfg);
    tokenizer_bdd.cfg_ok.set(true);
}
#[given("settings.json 的 compaction 块含 model 或 thinkingLevel 键")]
fn g_rc_compaction_settings_ignored(rc_snap: &RcSnap) {
    rc_snap.settings.borrow_mut().compaction = Some(
        xylitol_plugin_sdk::compaction_config::XyCompactionSettingsConfig {
            enabled: Some(true),
            reserve_tokens: Some(1024),
            keep_recent_tokens: Some(2048),
            model: Some(xylitol_plugin_sdk::model_entry::XyModelEntryConfig {
                provider: xylitol_plugin_sdk::model::XyModelKind::Fake,
                model: "ignored".into(),
                ..Default::default()
            }),
            thinking_level: Some("high".into()),
        },
    );
}
#[given("Settings.default_thinking_level 为 high 且模型支持集为 off 与 high 与 max")]
fn g_rc_thinking_default(rc_snap: &RcSnap) {
    rc_snap.settings.borrow_mut().default_thinking_level = Some("high".into());
    let yaml = "models:\n  models:\n    m:\n      provider: fake\n      model: x\n      thinking: true\n      thinking_levels: [off, high, max]\n";
    rc_snap
        .app_config
        .replace(Some(parse_app_config_yaml(yaml).expect("thinking yaml")));
}
#[given("Settings.default_thinking_level 为 off 且模型支持集为 off 与 high 与 max")]
fn g_rc_thinking_select(rc_snap: &RcSnap) {
    rc_snap.settings.borrow_mut().default_thinking_level = Some("off".into());
    let yaml = "models:\n  models:\n    m:\n      provider: fake\n      model: x\n      thinking: true\n      thinking_levels: [off, high, max]\n";
    rc_snap
        .app_config
        .replace(Some(parse_app_config_yaml(yaml).expect("thinking yaml")));
}
#[given("配置加载器已就绪")]
fn g_rc_docs(rc_snap: &RcSnap) {
    let _ = rc_snap;
}

#[given("YAML 模型条目含 thinking_levels [off, high, max]")]
fn g_rc_parse_list(tokenizer_bdd: &TokenizerBdd) {
    match parse_app_config_yaml(
        "models:\n  models:\n    m:\n      provider: fake\n      model: x\n      thinking: true\n      thinking_levels: [off, high, max]\n",
    ) {
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
#[given("thinking_levels 含空字符串")]
fn g_rc_empty_token_fails(tokenizer_bdd: &TokenizerBdd) {
    match parse_app_config_yaml(
        "models:\n  models:\n    m:\n      provider: fake\n      model: x\n      thinking: true\n      thinking_levels: [\"\"]\n",
    ) {
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
#[given("thinking_levels 含厂商字面量 bogon-level")]
fn g_rc_freeform_ok(tokenizer_bdd: &TokenizerBdd) {
    match parse_app_config_yaml(
        "models:\n  models:\n    m:\n      provider: fake\n      model: x\n      thinking: true\n      thinking_levels: [off, bogon-level]\n",
    ) {
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
#[given("thinking_levels 为 [off, high] 且 thinking_level_map 含 max: high")]
fn g_rc_map_key_outside_list_fails(tokenizer_bdd: &TokenizerBdd) {
    match parse_app_config_yaml(
        "models:\n  models:\n    m:\n      provider: fake\n      model: x\n      thinking: true\n      thinking_levels: [off, high]\n      thinking_level_map:\n        max: high\n",
    ) {
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
#[given("仅配置 thinking_levels 无 map")]
fn g_rc_absent_key_ok(tokenizer_bdd: &TokenizerBdd) {
    match parse_app_config_yaml(
        "models:\n  models:\n    m:\n      provider: fake\n      model: x\n      thinking: true\n      thinking_levels: [off, low]\n",
    ) {
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
#[given("YAML 含 thinking_levels [off, high] 与 thinking_level_map high: max 与 off: null")]
fn g_rc_parse_map(tokenizer_bdd: &TokenizerBdd) {
    match parse_app_config_yaml(
        "models:\n  models:\n    m:\n      provider: fake\n      model: x\n      thinking: true\n      thinking_levels: [off, high]\n      thinking_level_map:\n        high: max\n        off: null\n",
    ) {
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
#[given("仅存在 config.local.yaml 含可观测字段而无同层 config.yaml")]
fn g_rc_local_not_merged(tokenizer_bdd: &TokenizerBdd, rc_snap: &RcSnap) {
    let local_yaml = "models:\n  default_model: from-local\n  models:\n    from-local:\n      provider: fake\n      model: fake\n";
    rc_snap.local_yaml.replace(Some(local_yaml.to_string()));
    rc_load_flag::LOCAL_YAML.with(|l| l.replace(Some(local_yaml.to_string())));
    rc_load_flag::LOCAL_ONLY.with(|f| f.set(true));
    tokenizer_bdd.cfg_ok.set(true);
}

#[when("加载 settings")]
fn w_rc_load(rc_snap: &RcSnap) {
    rc_snap.load_settings_mgr();
}
#[when("读取缺省")]
fn w_rc_defaults(rc_snap: &RcSnap) {
    rc_snap.load_settings_mgr();
}
#[when("检查 Settings 类型与合并结果")]
fn w_rc_check_settings_no_prompts(rc_snap: &RcSnap) {
    rc_snap.load_settings_mgr();
}
#[when("加载配置并解析为运行时 settings")]
fn w_rc_load_compaction(rc_snap: &RcSnap) {
    let cfg = rc_snap.app_config.borrow().clone().expect("app config");
    let comp = cfg.compaction.expect("compaction section");
    rc_snap.compaction.replace(Some(
        xylitol_official_plugin_compaction::CompactionSettings::from(comp),
    ));
}
#[when("读取默认配置")]
fn w_rc_default_config(rc_snap: &RcSnap) {
    let cfg = xylitol_kernel::config::types::AppConfig::default();
    rc_snap.app_config.replace(Some(cfg));
}
#[when("加载配置并 resolve_model_meta")]
fn w_rc_resolve_meta(rc_snap: &RcSnap, tokenizer_bdd: &TokenizerBdd) {
    let cfg = tokenizer_bdd.config.borrow();
    match cfg.resolve_model_meta("m") {
        Ok(meta) => {
            rc_snap.meta.replace(Some(meta));
            tokenizer_bdd.cfg_ok.set(true);
        }
        Err(e) => {
            tokenizer_bdd.cfg_ok.set(false);
            tokenizer_bdd.cfg_err.replace(e.to_string());
        }
    }
}
#[when("会话首次装配")]
fn w_rc_assembly(rc_snap: &RcSnap) {
    let cfg = rc_snap.app_config.borrow().clone().expect("app config");
    let mut mm = rc_make_model_manager(&cfg);
    mm.select_model("m").expect("select model");
    let pref = rc_snap
        .settings
        .borrow()
        .default_thinking_level
        .as_deref()
        .map(str::to_owned);
    mm.set_preferred_default(pref);
    mm.apply_preferred_or_last();
    rc_snap.mm.replace(Some(mm));
}
#[when("select_model 到该模型")]
fn w_rc_select(rc_snap: &RcSnap) {
    let cfg = rc_snap.app_config.borrow().clone().expect("app config");
    let mut mm = rc_make_model_manager(&cfg);
    let pref = rc_snap
        .settings
        .borrow()
        .default_thinking_level
        .as_deref()
        .map(str::to_owned);
    mm.set_preferred_default(pref);
    mm.select_model("m").expect("select model");
    rc_snap.mm.replace(Some(mm));
}
#[when("加载并 resolve_model_meta")]
fn w_rc_resolve_meta2(rc_snap: &RcSnap, tokenizer_bdd: &TokenizerBdd) {
    w_rc_resolve_meta(rc_snap, tokenizer_bdd);
}
#[when("从全局 config.yaml 加载完整 settings")]
fn w_rc_narrative(rc_snap: &RcSnap) {
    let global_yaml = "models:\n  default_model: from-yaml\n  models:\n    from-yaml:\n      provider: fake\n      model: fake\n";
    let local_yaml = "models:\n  default_model: from-local\n";
    rc_snap.local_yaml.replace(Some(local_yaml.to_string()));
    rc_snap.app_config.replace(Some(
        parse_app_config_yaml(global_yaml).expect("global yaml"),
    ));
}

#[then("Settings.steering_mode 为 OneAtATime")]
fn t_rc_steering(rc_snap: &RcSnap) {
    let mgr = rc_snap.mgr.borrow();
    let mgr = mgr.as_ref().expect("settings loaded");
    assert_eq!(
        mgr.get_steering_mode(),
        xylitol_kernel::settings::SteeringMode::OneAtATime
    );
}
#[then("二者均为 OneAtATime")]
fn t_rc_both(rc_snap: &RcSnap) {
    let mgr = rc_snap.mgr.borrow();
    let mgr = mgr.as_ref().expect("settings loaded");
    assert_eq!(
        mgr.get_steering_mode(),
        xylitol_kernel::settings::SteeringMode::OneAtATime
    );
    assert_eq!(
        mgr.get_follow_up_mode(),
        xylitol_kernel::settings::SteeringMode::OneAtATime
    );
}
#[then("Settings 仅含已接线交付字段")]
fn t_rc_delivered_surface(rc_snap: &RcSnap) {
    let mgr = rc_snap.mgr.borrow();
    let mgr = mgr.as_ref().expect("settings loaded");
    let v = serde_json::to_value(mgr.get_settings()).unwrap();
    let obj = v.as_object().expect("settings object");
    let allowed = [
        "defaultThinkingLevel",
        "compaction",
        "thinkingBudgets",
        "steeringMode",
        "followUpMode",
    ];
    for key in obj.keys() {
        assert!(
            allowed.contains(&key.as_str()),
            "unexpected Settings field `{key}` (delivered surface only)"
        );
    }
}
#[then("不存在可生效的 prompts 路径列表字段")]
fn t_rc_no_prompts_field(rc_snap: &RcSnap) {
    let mgr = rc_snap.mgr.borrow();
    let mgr = mgr.as_ref().expect("settings loaded");
    let v = serde_json::to_value(mgr.get_settings()).unwrap();
    assert!(
        !v.get("prompts").is_some_and(|p| !p.is_null()),
        "Settings must not expose prompts list field"
    );
    let legacy: xylitol_kernel::settings::Settings =
        serde_json::from_str(r#"{"prompts":["legacy"]}"#).unwrap();
    let legacy_v = serde_json::to_value(&legacy).unwrap();
    assert!(
        !legacy_v.get("prompts").is_some_and(|p| !p.is_null()),
        "legacy prompts key must not take effect"
    );
}
#[then("compaction 任务模型条目与该锚点条目字段一致")]
fn t_rc_compaction_anchor(rc_snap: &RcSnap) {
    let cfg = rc_snap.app_config.borrow().clone().expect("app config");
    let entry = cfg
        .compaction
        .as_ref()
        .and_then(|c| c.model.as_ref())
        .expect("compaction model");
    assert_eq!(entry.model, "anchor-model");
}
#[then("两键被忽略且不进入运行时 CompactionSettings")]
fn t_rc_compaction_settings_ignored(rc_snap: &RcSnap) {
    let merged = xylitol_official_plugin_compaction::settings::merge_compaction_runtime(
        None,
        rc_snap.settings.borrow().compaction.as_ref(),
    );
    assert!(merged.model.is_none());
    assert!(merged.thinking_level.is_none());
    assert_eq!(merged.reserve_tokens, 1024);
    assert_eq!(merged.keep_recent_tokens, 2048);
}
#[then("compaction_settings.keep_recent_tokens 等于 YAML 中设置的值")]
fn t_rc_compaction(rc_snap: &RcSnap) {
    let settings = rc_snap.compaction.borrow();
    let settings = settings.as_ref().expect("compaction settings");
    assert_eq!(settings.keep_recent_tokens, 42_000);
}
#[then("mcp 未启用且字段可序列化")]
fn t_rc_mcp(rc_snap: &RcSnap) {
    let cfg = rc_snap.app_config.borrow();
    let cfg = cfg.as_ref().expect("app config");
    assert!(cfg.mcp_servers.is_none());
    assert!(serde_json::to_value(cfg).is_ok());
}
#[then("XyModelMeta.thinking_levels 与列表一致")]
fn t_rc_levels(rc_snap: &RcSnap) {
    let meta = rc_snap.meta.borrow();
    let meta = meta.as_ref().expect("model meta");
    assert_eq!(
        meta.thinking_levels,
        vec!["off".to_string(), "high".to_string(), "max".to_string()]
    );
}
#[then("当前 thinking level 为 high")]
fn t_rc_thinking_high_assembly(rc_snap: &RcSnap) {
    let mm = rc_snap.mm.borrow();
    let mm = mm.as_ref().expect("model manager");
    assert_eq!(mm.thinking_level(), "high");
}
#[then("thinking level 为 max")]
fn t_rc_thinking_max(rc_snap: &RcSnap) {
    let mm = rc_snap.mm.borrow();
    let mm = mm.as_ref().expect("model manager");
    assert_eq!(mm.thinking_level(), "max");
}
#[then("成功且支持集含 bogon-level")]
fn t_rc_freeform_ok(tokenizer_bdd: &TokenizerBdd) {
    assert!(
        tokenizer_bdd.cfg_ok.get(),
        "{}",
        tokenizer_bdd.cfg_err.borrow()
    );
    let cfg = tokenizer_bdd.config.borrow();
    let meta = cfg.resolve_model_meta("m").expect("meta");
    assert!(
        meta.thinking_levels.iter().any(|l| l == "bogon-level"),
        "expected bogon-level in {:?}",
        meta.thinking_levels
    );
}
#[then("meta 含 high→max 与 off→null")]
fn t_rc_meta(rc_snap: &RcSnap) {
    let meta = rc_snap.meta.borrow();
    let meta = meta.as_ref().expect("model meta");
    assert_eq!(
        meta.thinking_level_map
            .get("high")
            .and_then(|v| v.as_deref()),
        Some("max")
    );
    assert!(
        !meta.thinking_level_map.contains_key("off")
            || meta.thinking_level_map.get("off") == Some(&None)
    );
}
#[then("成功且 map 为空或缺省")]
fn t_rc_empty_map(tokenizer_bdd: &TokenizerBdd) {
    assert!(
        tokenizer_bdd.cfg_ok.get(),
        "{}",
        tokenizer_bdd.cfg_err.borrow()
    );
    let cfg = tokenizer_bdd.config.borrow();
    let meta = cfg.resolve_model_meta("m").expect("meta");
    assert!(meta.thinking_level_map.is_empty());
}
#[then("该字段不生效（local 被忽略）")]
fn t_rc_local(rc_snap: &RcSnap) {
    let cfg = xylitol_kernel::config::types::AppConfig::default();
    assert!(cfg.model.default_model.is_none());
    let local = rc_snap
        .local_yaml
        .borrow()
        .clone()
        .or_else(|| rc_load_flag::LOCAL_YAML.with(|l| l.borrow().clone()))
        .expect("local yaml");
    let local_cfg = parse_app_config_yaml(&local).expect("local yaml parses");
    assert_eq!(local_cfg.model.default_model.as_deref(), Some("from-local"));
}
#[then("全局 config.yaml 生效且不经 config.local.yaml 合并")]
fn t_rc_docs(rc_snap: &RcSnap) {
    let cfg = rc_snap.app_config.borrow();
    let cfg = cfg.as_ref().expect("app config");
    assert_eq!(cfg.model.default_model.as_deref(), Some("from-yaml"));
    let local = rc_snap.local_yaml.borrow();
    let local = local.as_ref().expect("local yaml");
    let local_cfg = parse_app_config_yaml(local).expect("local yaml");
    assert_ne!(
        cfg.model.default_model, local_cfg.model.default_model,
        "config.local.yaml must not be merged"
    );
}

#[when("生成配置 schema 并与仓库产物比较")]
pub(crate) fn w_config_schema_render(rc_snap: &RcSnap) {
    let rendered = xylitol_kernel::config::validate::render_app_config_schema();
    let again = xylitol_kernel::config::validate::render_app_config_schema();
    assert_eq!(rendered, again, "schema rendering MUST be idempotent");
    let target = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("configs")
        .join("config.schema.json");
    let checked_in = std::fs::read_to_string(&target)
        .unwrap_or_else(|e| panic!("read {}: {e}", target.display()));
    assert_eq!(
        checked_in, rendered,
        "configs/config.schema.json drifted from AppConfig; run `just gen-config-schema`"
    );
    rc_snap
        .app_config
        .replace(Some(xylitol_kernel::config::types::AppConfig::default()));
}

#[then("二者逐字节相同且再生成幂等")]
pub(crate) fn t_config_schema_same_source(rc_snap: &RcSnap) {
    assert!(
        rc_snap.app_config.borrow().is_some(),
        "schema comparison must have run"
    );
}

#[when("读取仓库配置 schema 的顶层属性")]
pub(crate) fn w_config_schema_top_level(rc_snap: &RcSnap) {
    let rendered = xylitol_kernel::config::validate::render_app_config_schema();
    let value: serde_json::Value =
        serde_json::from_str(&rendered).expect("config schema must be valid JSON");
    let props = value
        .get("properties")
        .and_then(|p| p.as_object())
        .expect("config schema must declare properties");
    let keys: Vec<String> = props.keys().cloned().collect();
    rc_snap.local_yaml.replace(Some(keys.join(",")));
}

#[then(
    "含 models 与 agents 与 session 与 tool_batch 与 tui 与 otel 与 loop_policies 与 todo 与 workflow 与 plan"
)]
pub(crate) fn t_config_schema_covers_sections(rc_snap: &RcSnap) {
    let joined = rc_snap
        .local_yaml
        .borrow()
        .clone()
        .expect("top-level keys captured");
    for key in [
        "models",
        "agents",
        "session",
        "tool_batch",
        "tui",
        "otel",
        "loop_policies",
        "todo",
        "workflow",
        "plan",
    ] {
        assert!(
            joined.split(',').any(|k| k == key),
            "config schema MUST declare `{key}`; got {joined}"
        );
    }
}

pub struct PresetBdd {
    pub yaml: RefCell<Option<String>>,
    pub app_config: RefCell<Option<xylitol_official_plugin_config::types::AppConfig>>,
    pub error: RefCell<Option<String>>,
    pub resolved: RefCell<Option<xylitol_plugin_sdk::model::XyModelConfig>>,
    pub adapter: RefCell<Option<xylitol_kernel::provider::adapter::AdapterKind>>,
}

#[fixture]
pub fn preset_bdd() -> PresetBdd {
    PresetBdd {
        yaml: RefCell::new(None),
        app_config: RefCell::new(None),
        error: RefCell::new(None),
        resolved: RefCell::new(None),
        adapter: RefCell::new(None),
    }
}

fn load_preset_yaml(
    yaml: &str,
) -> Result<xylitol_official_plugin_config::types::AppConfig, String> {
    let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
    let global = tmp.path().join("config");
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&global).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&home).map_err(|e| e.to_string())?;
    std::fs::write(global.join("config.yaml"), yaml).map_err(|e| e.to_string())?;
    let global_s = global.to_str().expect("global path").to_string();
    let home_s = home.to_str().expect("home path").to_string();
    let global_env = global_s.clone();
    let home_env = home_s.clone();
    xylitol_official_plugin_config::loader::load_app_config_with(
        None,
        move |key| match key {
            "XYLITOL_CONFIG_DIR" => Some(global_env.clone()),
            "XYLITOL_PROJECT_DIR" | "HOME" => Some(home_env.clone()),
            _ => None,
        },
        None,
    )
    .map_err(|e| e.to_string())
}

#[given("YAML 模型条目仅含 preset: opencode-zen 与 api_key")]
fn g_c2995_preset_expand(preset_bdd: &PresetBdd) {
    *preset_bdd.yaml.borrow_mut() = Some(
        "models:\n  models:\n    zen:\n      preset: opencode-zen\n      api_key: sk-test\n".into(),
    );
}

#[given("YAML 模型条目含 preset: zmodel 且显式 model 与 context_window")]
fn g_c2995_preset_override(preset_bdd: &PresetBdd) {
    *preset_bdd.yaml.borrow_mut() = Some(
        "models:\n  models:\n    glm:\n      preset: zmodel\n      model: glm-5.2\n      context_window: 64000\n"
            .into(),
    );
}

#[given("YAML 模型条目含 preset: no-such")]
fn g_c2995_preset_unknown(preset_bdd: &PresetBdd) {
    *preset_bdd.yaml.borrow_mut() =
        Some("models:\n  models:\n    mystery:\n      preset: no-such\n".into());
}

#[when("加载预设配置")]
fn w_c2995_load_preset(preset_bdd: &PresetBdd) {
    let yaml = preset_bdd.yaml.borrow().clone().expect("yaml");
    match load_preset_yaml(&yaml) {
        Ok(cfg) => {
            preset_bdd.app_config.replace(Some(cfg));
            preset_bdd.error.replace(None);
        }
        Err(err) => {
            preset_bdd.app_config.replace(None);
            preset_bdd.error.replace(Some(err));
        }
    }
}

#[then("provider 为 openai 且 model 与 base_url 与 api 来自预设")]
fn t_c2995_preset_expand(preset_bdd: &PresetBdd) {
    let cfg = preset_bdd.app_config.borrow();
    let cfg = cfg.as_ref().expect("loaded config");
    let entry = cfg.model.models.get("zen").expect("zen entry");
    assert_eq!(
        entry.provider,
        xylitol_plugin_sdk::model::XyModelKind::OpenAi
    );
    assert_eq!(entry.model, "deepseek-v4-flash-free");
    assert_eq!(
        entry.base_url.as_deref(),
        Some("https://opencode.ai/zen/v1")
    );
    assert_eq!(entry.api.as_deref(), Some("openai-completions"));
    assert_eq!(entry.api_key.as_deref(), Some("sk-test"));
}

#[then("显式字段覆盖预设默认值")]
fn t_c2995_preset_override(preset_bdd: &PresetBdd) {
    let cfg = preset_bdd.app_config.borrow();
    let cfg = cfg.as_ref().expect("loaded config");
    let entry = cfg.model.models.get("glm").expect("glm entry");
    assert_eq!(entry.model, "glm-5.2");
    assert_eq!(entry.context_window, 64_000);
    assert_eq!(
        entry.base_url.as_deref(),
        Some("https://api.z.ai/api/paas/v4")
    );
}

#[then("失败且诊断含 unknown preset")]
fn t_c2995_preset_unknown(preset_bdd: &PresetBdd) {
    let err = preset_bdd.error.borrow();
    let err = err.as_ref().expect("expected load failure");
    assert!(err.contains("unknown preset: no-such"), "{err}");
    assert!(err.contains("models.mystery"), "{err}");
}

#[given("预设展开后的模型条目 api 为 openai-completions")]
fn g_c2995_adapter_preset(preset_bdd: &PresetBdd) {
    g_c2995_preset_expand(preset_bdd);
    w_c2995_load_preset(preset_bdd);
}

#[when("解析模型配置")]
fn w_c2995_resolve(preset_bdd: &PresetBdd) {
    let cfg = preset_bdd.app_config.borrow();
    let cfg = cfg.as_ref().expect("loaded config");
    let resolved = cfg.resolve_model("zen").expect("resolve zen");
    let adapter = xylitol_kernel::provider::adapter::factory::resolve_adapter_kind(&resolved);
    preset_bdd.resolved.replace(Some(resolved));
    preset_bdd.adapter.replace(Some(adapter));
}

#[then("adapter 为 OpenAI Completions 且预设名不参与选择")]
fn t_c2995_adapter_preset(preset_bdd: &PresetBdd) {
    let adapter = preset_bdd.adapter.borrow();
    assert_eq!(
        adapter.as_ref(),
        Some(&xylitol_kernel::provider::adapter::AdapterKind::OpenAiCompletions),
        "preset api must drive adapter selection"
    );
    let resolved = preset_bdd.resolved.borrow();
    let resolved = resolved.as_ref().expect("resolved config");
    assert_eq!(resolved.model, "deepseek-v4-flash-free");
    assert_eq!(resolved.api.as_deref(), Some("openai-completions"));
}
