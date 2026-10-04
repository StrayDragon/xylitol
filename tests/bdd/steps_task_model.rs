//! BDD steps for runtime-model-registry m18 (task-level model resolution).

use crate::tests::bdd::fixtures::*;
use crate::tests::bdd::helpers::result_ok_str;
use crate::tests::bdd::prelude::*;
use rstest_bdd_macros::{given, then, when};

#[given("任务模型条目可构建")]
pub(crate) fn g_m18_task_entry_ok(agent: &AgentState) {
    agent
        .compaction_task_model
        .replace(Some(crate::protocol::model_entry::XyModelEntryConfig {
            provider: crate::protocol::model::XyModelKind::Fake,
            model: "summary-task".into(),
            thinking: false,
            ..Default::default()
        }));
}

#[given("任务模型条目构建失败")]
pub(crate) fn g_m18_task_entry_fail(agent: &AgentState) {
    agent
        .compaction_task_model
        .replace(Some(crate::protocol::model_entry::XyModelEntryConfig {
            provider: crate::protocol::model::XyModelKind::Fake,
            model: "bad-summary".into(),
            thinking: true,
            thinking_levels: Some(vec!["".into()]),
            ..Default::default()
        }));
}

#[when("经任务级解析入口取模型")]
pub(crate) fn w_m18_resolve(agent: &AgentState) {
    use std::sync::Arc;

    let mut reg = ModelRegistry::new();
    let meta = crate::protocol::model::XyModelMeta {
        id: "main".into(),
        config: crate::protocol::model::XyModelConfig {
            kind: crate::protocol::model::XyModelKind::Fake,
            api_key: String::new(),
            model: "main-wire".into(),
            base_url: None,
            api: None,
            compat: None,
        },
        display_name: "main".into(),
        thinking: true,
        context_window: 128_000,
        api: String::new(),
        provider: "fake".into(),
        cost_input: 0.0,
        cost_output: 0.0,
        cost_cache_read: 0.0,
        cost_cache_write: 0.0,
        max_tokens: 0,
        thinking_levels: vec!["off".into(), "high".into()],
        thinking_level_map: Default::default(),
    };
    reg.register(meta);
    let mut mm = crate::agent::model::ModelManager::new(
        reg,
        Arc::new(crate::infra::provider::factory::build_provider),
    );
    mm.select_model("main").expect("select main");

    let settings = crate::agent::compaction::CompactionSettings {
        model: agent.compaction_task_model.borrow().clone(),
        ..Default::default()
    };
    let binding = crate::agent::model::task_model::resolve_compaction_summary(
        &settings,
        &mm,
        &Default::default(),
    )
    .expect("resolve");
    agent.last_result.replace(Some(Ok(format!(
        "actual={};fallback={};requested={}",
        binding.attribution.actual_model,
        binding.attribution.fallback,
        binding
            .attribution
            .requested_model
            .as_deref()
            .unwrap_or_default(),
    ))));
    agent.compaction_binding.replace(Some(binding));
}

#[then("返回独立实例且其模型标识与条目一致")]
pub(crate) fn t_m18_independent(agent: &AgentState) {
    let text = result_ok_str(&agent.last_result);
    assert!(text.contains("actual=summary-task"));
    assert!(text.contains("fallback=false"));
    let binding = agent.compaction_binding.borrow();
    let binding = binding.as_ref().expect("binding");
    assert_eq!(binding.meta.config.model, "summary-task");
}

#[then("回退当前会话模型且回退态可观测")]
pub(crate) fn t_m18_fallback(agent: &AgentState) {
    let text = result_ok_str(&agent.last_result);
    assert!(text.contains("actual=main-wire"));
    assert!(text.contains("fallback=true"));
    let binding = agent.compaction_binding.borrow();
    let binding = binding.as_ref().expect("binding");
    assert!(binding.attribution.notice_message().is_some());
    let obs = binding.attribution.enrich_obs(&Default::default());
    assert_eq!(obs.compaction_model_fallback, Some(true));
}

// ── c2835 后继：runtime-model-registry 裸规则回填（结构探针）────────

// 结构探针：provider 缺省优先级 / manifest 缺省协议族。
thread_local! {
    pub(crate) static MODEL_PROBE: RefCell<Option<(u32, u32, u32)>> = const { RefCell::new(None) };
    pub(crate) static MODEL_KIND_PROBE: RefCell<Option<(usize, usize)>> = const { RefCell::new(None) };
}

#[when("读取内置 provider 默认表")]
pub(crate) fn w_read_provider_builtin_defaults() {
    use crate::agent::model::registry::{ProviderApi, ProviderConfig};
    let openai = ProviderConfig::openai(Some("k".into()));
    let anthropic = ProviderConfig::anthropic(Some("k".into()));
    let custom = ProviderConfig::custom("lm", ProviderApi::OpenAiCompatible, "http://x", None);
    MODEL_PROBE.with(|p| {
        *p.borrow_mut() = Some((openai.priority, anthropic.priority, custom.priority));
    });
}

#[then("内置默认仅两族且优先级有序")]
pub(crate) fn t_provider_builtin_defaults_ordered() {
    let (o, a, c) = MODEL_PROBE
        .with(|p| *p.borrow())
        .expect("provider 探针已跑");
    assert!(
        o < a && a < c,
        "内置两族 MUST 先于自定义 provider，实得 {o}/{a}/{c}"
    );
}

#[when("以 provider 种类读取适配器缺省")]
pub(crate) fn w_read_adapter_kind_default() {
    let text = std::fs::read_to_string("src/infra/provider/adapter/mod.rs")
        .expect("src/infra/provider/adapter/mod.rs 可读");
    let anthropic_hit =
        text.contains("\"anthropic-messages\" => AdapterKind::AnthropicMessages") as usize;
    let openai_hit = text.contains("=> AdapterKind::OpenAiResponses") as usize;
    MODEL_KIND_PROBE.with(|p| *p.borrow_mut() = Some((anthropic_hit, openai_hit)));
}

#[then("manifest 缺省按 provider 选协议族")]
pub(crate) fn t_adapter_kind_default_by_provider() {
    let (anthropic_hit, openai_hit) = MODEL_KIND_PROBE.with(|p| *p.borrow()).expect("探针已跑");
    assert_eq!(
        anthropic_hit, 1,
        "anthropic MUST 缺省为 anthropic-messages 族"
    );
    assert!(
        openai_hit >= 1,
        "其余 provider MUST 缺省落 openai-responses 族（fallback 分支）"
    );
}
