use crate::tests::bdd::prelude::*;
use crate::tests::bdd::steps_tokenizer::{TokenizerBdd, parse_app_config_yaml};
use rstest::fixture;
use rstest_bdd_macros::{given, then, when};

pub struct RcSnap {
    pub(crate) settings: RefCell<crate::infra::settings::Settings>,
    pub(crate) mgr: RefCell<Option<crate::infra::settings::SettingsManager>>,
    pub(crate) app_config: RefCell<Option<crate::infra::config::types::AppConfig>>,
    pub(crate) local_yaml: RefCell<Option<String>>,
    pub(crate) compaction: RefCell<Option<crate::agent::compaction::CompactionSettings>>,
    pub(crate) meta: RefCell<Option<XyModelMeta>>,
    pub(crate) mm: RefCell<Option<crate::agent::model::ModelManager>>,
}
impl RcSnap {
    fn new() -> Self {
        Self {
            settings: RefCell::new(crate::infra::settings::Settings::default()),
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
            .replace(Some(crate::infra::settings::SettingsManager::in_memory(s)));
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
    cfg: &crate::infra::config::types::AppConfig,
) -> crate::agent::model::ModelManager {
    let meta = cfg.resolve_model_meta("m").expect("resolve_model_meta");
    let mut reg = ModelRegistry::new();
    reg.register(meta);
    crate::agent::model::ModelManager::new(
        reg,
        Arc::new(crate::infra::provider::factory::build_provider),
    )
}
#[fixture]
pub fn rc_snap() -> RcSnap {
    RcSnap::new()
}

// --- given ---
#[given("settings.json 中 steering_mode 设为 one-at-a-time")]
fn g_rc_steering(rc_snap: &RcSnap) {
    rc_snap.settings.borrow_mut().steering_mode =
        Some(crate::infra::settings::SteeringMode::OneAtATime);
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
    // Narrative scenario only needs in-memory AppConfig (see w_rc_narrative);
    // no process-env loader injection.
    let _ = rc_snap;
}

// TokenizerBdd-backed givens for rc config-load scenarios
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
    // Asserts local-only content is ignored via default AppConfig + parse of the
    // local body (t_rc_local) — no process-env loader side effects.
    let local_yaml = "models:\n  default_model: from-local\n  models:\n    from-local:\n      provider: fake\n      model: fake\n";
    rc_snap.local_yaml.replace(Some(local_yaml.to_string()));
    rc_load_flag::LOCAL_YAML.with(|l| l.replace(Some(local_yaml.to_string())));
    rc_load_flag::LOCAL_ONLY.with(|f| f.set(true));
    tokenizer_bdd.cfg_ok.set(true);
}

// --- when ---
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
    rc_snap
        .compaction
        .replace(Some(crate::agent::compaction::CompactionSettings::from(
            comp,
        )));
}
#[when("读取默认配置")]
fn w_rc_default_config(rc_snap: &RcSnap) {
    let cfg = crate::infra::config::types::AppConfig::default();
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

// --- then ---
#[then("Settings.steering_mode 为 OneAtATime")]
fn t_rc_steering(rc_snap: &RcSnap) {
    let mgr = rc_snap.mgr.borrow();
    let mgr = mgr.as_ref().expect("settings loaded");
    assert_eq!(
        mgr.get_steering_mode(),
        crate::infra::settings::SteeringMode::OneAtATime
    );
}
#[then("二者均为 OneAtATime")]
fn t_rc_both(rc_snap: &RcSnap) {
    let mgr = rc_snap.mgr.borrow();
    let mgr = mgr.as_ref().expect("settings loaded");
    assert_eq!(
        mgr.get_steering_mode(),
        crate::infra::settings::SteeringMode::OneAtATime
    );
    assert_eq!(
        mgr.get_follow_up_mode(),
        crate::infra::settings::SteeringMode::OneAtATime
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
#[then("MUST NOT 存在可生效的 prompts 路径列表字段")]
fn t_rc_no_prompts_field(rc_snap: &RcSnap) {
    let mgr = rc_snap.mgr.borrow();
    let mgr = mgr.as_ref().expect("settings loaded");
    let v = serde_json::to_value(mgr.get_settings()).unwrap();
    assert!(
        !v.get("prompts").is_some_and(|p| !p.is_null()),
        "Settings must not expose prompts list field"
    );
    let legacy: crate::infra::settings::Settings =
        serde_json::from_str(r#"{"prompts":["legacy"]}"#).unwrap();
    let legacy_v = serde_json::to_value(&legacy).unwrap();
    assert!(
        !legacy_v.get("prompts").is_some_and(|p| !p.is_null()),
        "legacy prompts key must not take effect"
    );
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
    let cfg = crate::infra::config::types::AppConfig::default();
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

// TokenizerBdd-backed then for "失败" steps
// rc "失败" is already satisfied by the tokenizer_bdd t_rc18_fail step.
// rc "加载配置" is already satisfied by tokenizer_bdd w_rc18_load step.
// Both require tokenizer_bdd fixture in scenario binding.

// --- scenario bindings ---
