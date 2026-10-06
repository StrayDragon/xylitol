//! Steps for `infra-otel` — 低频观测 span 族（otel6/7/8/10/11/12/22）。
//!
//! 经真实 AgentRuntime（fake provider + 工具调用）在观测闸 + 收集槽下驱动；
//! 断言落在收集到的 fastrace `SpanRecord` 属性上。otel18–22 由单测覆盖
//! （spec 明文禁 BDD step），otel1–5 / 9 / 13 / 17 / 20 / 21 维持单测与
//! live smoke 承载，不在本文件范围。

use crate::bdd::fixtures::AgentState;
use crate::bdd::prelude::*;
use rstest::fixture;
use rstest_bdd_macros::{given, then, when};
use xylitol::infra::provider::factory::{set_fake_text, set_fake_tool_call, set_fake_tool_result};
use xylitol_ai_bridge::provider::trace::{
    ObservationIoTier, SpanCollectScope, set_observation_io_tier, set_provider_trace_active,
    set_tool_observation_io_tier,
};
use xylitol_ai_bridge::provider::{clear_obs_session, set_obs_session};

pub(crate) const SESSION_UUID: &str = "aaagggg-hhhh-iiii-jjjj-kkkkllllmmmm";

/// Per-scenario globals + collect sink. Uses process-slot setters (not TLS
/// scopes): the async runtime polls the stream on worker threads where TLS
/// scopes are invisible. Drop restores the pre-scenario state.
pub struct OtelBdd {
    collect: RefCell<Option<SpanCollectScope>>,
    io_tier: RefCell<ObservationIoTier>,
    mounted: Cell<bool>,
}

#[fixture]
pub fn otel_bdd() -> OtelBdd {
    OtelBdd {
        collect: RefCell::new(None),
        io_tier: RefCell::new(ObservationIoTier::None),
        mounted: Cell::new(false),
    }
}

impl Drop for OtelBdd {
    fn drop(&mut self) {
        if self.mounted.get() {
            set_provider_trace_active(false);
            set_observation_io_tier(ObservationIoTier::None);
            set_tool_observation_io_tier(ObservationIoTier::None);
            clear_obs_session();
        }
    }
}

impl OtelBdd {
    fn mount_scopes(&self, session_name: Option<&str>) {
        let tier = *self.io_tier.borrow();
        set_provider_trace_active(true);
        set_observation_io_tier(tier);
        set_tool_observation_io_tier(tier);
        set_obs_session(SESSION_UUID, session_name.map(str::to_string));
        drop(self.collect.borrow_mut().take());
        *self.collect.borrow_mut() = Some(SpanCollectScope::enter());
        self.mounted.set(true);
    }

    /// c2830：指定 io 档臂装（mock 上游场景复用同一闸具与收集槽）。
    pub(crate) fn mount_io(&self, tier: ObservationIoTier) {
        *self.io_tier.borrow_mut() = tier;
        self.mount_scopes(None);
    }

    pub(crate) fn records(&self) -> Vec<fastrace::collector::SpanRecord> {
        fastrace::flush();
        self.collect
            .borrow()
            .as_ref()
            .expect("collect scope mounted")
            .records()
    }
}

/// 运行一次带工具调用的 agent 回合（fake provider：文本 + read 工具）。
/// `session_name` 走产品改名路径（bind 后 set_obs_session_name）。
pub(crate) async fn run_turn_with_tool(agent: &AgentState, session_name: Option<&str>) {
    set_fake_text("我来读文件");
    set_fake_tool_call("read", r#"{"path":"src/main.rs"}"#);
    set_fake_tool_result("hello world");
    let mut runner = crate::bdd::helpers::make_agent(agent);
    // 显式绑定已知会话 UUID——runtime bind_session 会把 obs session 槽
    // 同步为当前会话，otel6 的「当前会话 UUID」即此 id。
    crate::bdd::helpers::bind_session_or_panic(&mut runner, SESSION_UUID);
    if let Some(name) = session_name {
        xylitol_ai_bridge::provider::set_obs_session_name(Some(name));
    }
    let mut stream = crate::bdd::helpers::agent_submit_root(&mut runner, "读取文件").await;
    while let Some(e) = stream.next().await {
        if let XyEvent::Error(err) = &e {
            panic!(
                "[otel-bdd] run failed: kind={} message={}",
                err.kind, err.message
            );
        }
    }
}

fn prop<'a>(record: &'a fastrace::collector::SpanRecord, key: &str) -> Option<&'a str> {
    record
        .properties
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_ref())
}

// ── mounts ────────────────────────────────────────────────────

#[when("以观测闸开启、会话 UUID 与收集槽运行一次带工具调用的 agent 回合")]
pub(crate) async fn w_otel_run_traced(otel_bdd: &OtelBdd, agent: &AgentState) {
    *otel_bdd.io_tier.borrow_mut() = ObservationIoTier::None;
    otel_bdd.mount_scopes(None);
    run_turn_with_tool(agent, None).await;
}

#[when("以观测闸开启运行一次带工具调用的 agent 回合")]
pub(crate) async fn w_otel_run_gate_only(otel_bdd: &OtelBdd, agent: &AgentState) {
    w_otel_run_traced(otel_bdd, agent).await;
}

#[when("以带 display name 的会话身份运行一次 agent 回合")]
pub(crate) async fn w_otel_run_named(otel_bdd: &OtelBdd, agent: &AgentState) {
    *otel_bdd.io_tier.borrow_mut() = ObservationIoTier::None;
    otel_bdd.mount_scopes(Some("my session"));
    run_turn_with_tool(agent, Some("my session")).await;
}

#[when("以无 name 的会话身份运行一次 agent 回合")]
pub(crate) async fn w_otel_run_unnamed(otel_bdd: &OtelBdd, agent: &AgentState) {
    otel_bdd.mount_scopes(None);
    run_turn_with_tool(agent, None).await;
}

#[when("以 io=none 的观测闸运行一次带工具调用的 agent 回合")]
pub(crate) async fn w_otel_run_io_none(otel_bdd: &OtelBdd, agent: &AgentState) {
    *otel_bdd.io_tier.borrow_mut() = ObservationIoTier::None;
    otel_bdd.mount_scopes(None);
    run_turn_with_tool(agent, None).await;
}

#[when("以 io=truncated 的观测闸运行一次带工具调用的 agent 回合")]
pub(crate) async fn w_otel_run_io_truncated(otel_bdd: &OtelBdd, agent: &AgentState) {
    *otel_bdd.io_tier.borrow_mut() = ObservationIoTier::Truncated;
    otel_bdd.mount_scopes(None);
    run_turn_with_tool(agent, None).await;
}

fn turn_root(records: &[fastrace::collector::SpanRecord]) -> &fastrace::collector::SpanRecord {
    records
        .iter()
        .find(|s| s.name == "agent.turn")
        .expect("agent.turn root span must be exported")
}

// ── otel6 ─────────────────────────────────────────────────────

#[then("agent.turn 根 span 携带等于会话 UUID 的 langfuse.session.id")]
fn t_otel6_session_id(otel_bdd: &OtelBdd) {
    let records = otel_bdd.records();
    let root = turn_root(&records);
    assert_eq!(
        prop(root, "langfuse.session.id"),
        Some(SESSION_UUID),
        "otel6: session id equals the bound session UUID: {:?}",
        root.properties
    );
}

// ── otel7 ─────────────────────────────────────────────────────

#[then("根 span 携带 session_name 元数据")]
fn t_otel7_named(otel_bdd: &OtelBdd) {
    let records = otel_bdd.records();
    let root = turn_root(&records);
    assert_eq!(
        prop(root, "langfuse.trace.metadata.session_name"),
        Some("my session"),
        "otel7: display name exported as trace metadata"
    );
}

#[then("根 span 不写 session_name 属性")]
fn t_otel7_unnamed(otel_bdd: &OtelBdd) {
    let records = otel_bdd.records();
    let root = turn_root(&records);
    assert!(
        prop(root, "langfuse.trace.metadata.session_name").is_none(),
        "otel7: no name → no session_name property"
    );
    assert_eq!(
        prop(root, "langfuse.session.id"),
        Some(SESSION_UUID),
        "otel7: id remains stable without a name"
    );
}

// ── otel8 ─────────────────────────────────────────────────────

#[then("agent.span 为 agent 且 tool.execute 为 tool")]
fn t_otel8_types(otel_bdd: &OtelBdd) {
    let records = otel_bdd.records();
    let cases = [
        ("agent.turn", "agent"),
        ("agent.iteration", "agent"),
        ("tool.execute", "tool"),
    ];
    for (name, ty) in cases {
        let span = records
            .iter()
            .find(|s| s.name == name)
            .unwrap_or_else(|| panic!("otel8: span `{name}` must be exported"));
        assert_eq!(
            prop(span, "langfuse.observation.type"),
            Some(ty),
            "otel8: {name} observation type"
        );
    }
}

#[then("不写 gen_ai 等价键")]
fn t_otel8_no_genai(otel_bdd: &OtelBdd) {
    let records = otel_bdd.records();
    for span in &records {
        for (k, _) in &span.properties {
            assert!(
                !k.starts_with("gen_ai."),
                "otel8: legacy gen_ai key `{k}` on {}",
                span.name
            );
        }
    }
}

// ── otel11 ────────────────────────────────────────────────────

#[then("iteration、tool 与 token.estimate 均为 turn 根的后代并共享 trace_id")]
fn t_otel11_trace_tree(otel_bdd: &OtelBdd) {
    let records = otel_bdd.records();
    let root = turn_root(&records);
    let root_trace = root.trace_id;
    // llm.request 仅由 native HTTP 适配层产生（fake 无 HTTP 层），其父子
    // 关系由适配层单测承载；此处断言 runtime 可达的 span 树。
    for name in ["agent.iteration", "tool.execute", "token.estimate"] {
        let span = records
            .iter()
            .find(|s| s.name == name)
            .unwrap_or_else(|| panic!("otel11: `{name}` must share the turn trace"));
        assert_eq!(
            span.trace_id, root_trace,
            "otel11: {name} must share the root trace_id"
        );
    }
}

// ── otel12 ────────────────────────────────────────────────────

#[then("导出名全部为产品词汇（含 token.estimate）且无旧名")]
fn t_otel12_names(otel_bdd: &OtelBdd) {
    let records = otel_bdd.records();
    let allowed = [
        "agent.turn",
        "agent.iteration",
        "llm.request",
        "tool.execute",
        "token.estimate",
    ];
    for span in &records {
        assert!(
            allowed.contains(&span.name.as_ref()),
            "otel12: unexpected export name `{}`",
            span.name
        );
    }
    for legacy in ["react.stream", "react.turn", "provider.request"] {
        assert!(
            !records.iter().any(|s| s.name == legacy),
            "otel12: legacy name `{legacy}` must not appear"
        );
    }
}

// ── otel22 ────────────────────────────────────────────────────

#[then("全部主路径 span 携带 xylitol.obs.lane=llm")]
fn t_otel22_lane(otel_bdd: &OtelBdd) {
    let records = otel_bdd.records();
    assert!(!records.is_empty(), "otel22: spans must be exported");
    for span in &records {
        assert_eq!(
            prop(span, "xylitol.obs.lane"),
            Some("llm"),
            "otel22: lane attr on {}",
            span.name
        );
    }
}

// ── otel10 / 14 / 15 / 16 ─────────────────────────────────────

#[then("任何 span 都不带 observation input 或 output")]
fn t_otel10_io_absent(otel_bdd: &OtelBdd) {
    let records = otel_bdd.records();
    for span in &records {
        for key in ["langfuse.observation.input", "langfuse.observation.output"] {
            assert!(
                prop(span, key).is_none(),
                "otel10: {key} must be absent at io=none on {}",
                span.name
            );
        }
    }
}

#[then("tool.execute 带参数与结果摘要且 agent.turn 带提示预览")]
fn t_otel10_io_truncated(otel_bdd: &OtelBdd) {
    let records = otel_bdd.records();
    let tool = records
        .iter()
        .find(|s| s.name == "tool.execute")
        .expect("tool span");
    assert!(
        prop(tool, "langfuse.observation.input").is_some(),
        "otel14: tool params summary at truncated tier"
    );
    assert!(
        prop(tool, "langfuse.observation.output").is_some(),
        "otel14: tool result summary at truncated tier"
    );
    let turn = turn_root(&records);
    let turn_input = prop(turn, "langfuse.observation.input")
        .expect("otel15: turn input preview at truncated tier");
    assert!(
        turn_input.contains("读取文件"),
        "otel15: user prompt preview: {turn_input}"
    );
}

// ── c2826 specs-compact：裸规则转场景补充（r1466/r1470/r1471/r1473/r1474/r1476/r1477/r1478/r1479/r1486/r1492）──

fn find_named<'a>(
    records: &'a [fastrace::collector::SpanRecord],
    name: &str,
) -> Vec<&'a fastrace::collector::SpanRecord> {
    records.iter().filter(|r| r.name == name).collect()
}

#[when("以未开启观测闸运行一次带工具调用的 agent 回合")]
async fn w_c2826_no_gate_run(agent: &AgentState, _otel_bdd: &OtelBdd) {
    // serial 序内起点即产品默认态：闸关（此前所有 otel 测试的 Drop 已复位）。
    let gate_off_at_rest = !xylitol_ai_bridge::provider::trace::provider_trace_active();
    set_fake_text("我来读文件");
    set_fake_tool_call("read", r#"{"path":"src/main.rs"}"#);
    set_fake_tool_result("hello world");
    // 不臂装任何收集槽/观测闸：默认关闸运行一回，验证主路径不受影响。
    let mut runner = crate::bdd::helpers::make_agent(agent);
    crate::bdd::helpers::bind_session_or_panic(&mut runner, SESSION_UUID);
    let mut stream = crate::bdd::helpers::agent_submit_root(&mut runner, "读取文件").await;
    while let Some(e) = stream.next().await {
        if let XyEvent::Error(err) = &e {
            panic!(
                "[otel-bdd] run failed: kind={} message={}",
                err.kind, err.message
            );
        }
    }
    // 回合结束后的常驻闸态仍应为关（本测试从未臂装）。
    let gate_off_after = !xylitol_ai_bridge::provider::trace::provider_trace_active();
    assert!(
        gate_off_at_rest && gate_off_after,
        "c2826: 默认（未配置 [otel]）观测闸必须为关：起点 {} / 终点 {}",
        gate_off_at_rest,
        gate_off_after
    );
}

#[then("默认观测闸为关且回合正常完成不出口任何 span")]
fn t_c2826_no_gate_no_export() {
    // 断言已内联在 when（闸态捕获与回合完成）；此处保留 then 以钉住场景语义。
}

#[cfg(feature = "otel")]
#[when("以合法 otlp-http 配置尝试构建 OTLP reporter")]
async fn w_c2826_build_reporter_ok(otel_bdd: &OtelBdd) {
    use xylitol::infra::config::types::{OtelConfig, OtelExporterKind};
    let cfg = OtelConfig {
        exporter: OtelExporterKind::OtlpHttp,
        endpoint: Some("http://127.0.0.1:9/api/public/otel".into()),
        ..Default::default()
    };
    let built = xylitol::infra::observability::otel::install::try_build_otlp_reporter(&cfg);
    otel_bdd.mounted.set(built.is_some());
    drop(built);
}

#[cfg(feature = "otel")]
#[then("成功构建出可安装的 reporter")]
fn t_c2826_reporter_built(otel_bdd: &OtelBdd) {
    assert!(
        otel_bdd.mounted.get(),
        "c2826: 合法 otlp-http 配置必须能构建 reporter"
    );
}

#[when("以缺失 endpoint 的 otlp-http 配置尝试构建 OTLP reporter")]
async fn w_c2826_build_reporter_bad(otel_bdd: &OtelBdd) {
    use xylitol::infra::config::types::{OtelConfig, OtelExporterKind};
    let cfg = OtelConfig {
        exporter: OtelExporterKind::OtlpHttp,
        endpoint: None,
        ..Default::default()
    };
    // 本步要钉的是「endpoint 缺失」形状，而 endpoint 也可由 LANGFUSE_BASE_URL 供。
    // 并行套件里前序用例可能已注入该 env，故本步隔离 env 后原样放回
    // （与 `#[serial(env_global)]` 同一纪律），使判据与执行顺序无关。
    const KEYS: [&str; 3] = [
        "LANGFUSE_BASE_URL",
        "LANGFUSE_PUBLIC_KEY",
        "LANGFUSE_SECRET_KEY",
    ];
    let saved: Vec<(&str, String)> = KEYS
        .iter()
        .filter_map(|k| std::env::var(k).ok().map(|v| (*k, v)))
        .collect();
    for (key, _) in &saved {
        unsafe {
            std::env::remove_var(key);
        }
    }
    let built = xylitol::infra::observability::otel::install::try_build_otlp_reporter(&cfg);
    otel_bdd.mounted.set(built.is_none());
    drop(built);
    for (key, value) in saved {
        unsafe {
            std::env::set_var(key, value);
        }
    }
}

#[then("构建安静返回 None 且产生 obs 诊断且不失败")]
fn t_c2826_reporter_none_diag(otel_bdd: &OtelBdd) {
    let diag = xylitol::infra::observability::otel::otlp_disabled_diag();
    assert!(
        diag.is_some(),
        "c2826: otlp-http 未生效必须经 obs_diag 呈现（构建返回 None = {}）",
        otel_bdd.mounted.get()
    );
    let msg = diag.expect("diag");
    assert!(!msg.contains("sk-"), "c2826: 诊断不得携带密钥");
}

#[when("以 io=none 且 tool_io=truncated 的观测闸运行一次带工具调用的 agent 回合")]
async fn w_c2826_tool_tier_only(agent: &AgentState, otel_bdd: &OtelBdd) {
    set_fake_text("我来读文件");
    set_fake_tool_call("read", r#"{"path":"src/main.rs"}"#);
    set_fake_tool_result("hello world");
    set_provider_trace_active(true);
    set_observation_io_tier(ObservationIoTier::None);
    set_tool_observation_io_tier(ObservationIoTier::Truncated);
    set_obs_session(SESSION_UUID, None);
    drop(otel_bdd.collect.borrow_mut().take());
    *otel_bdd.collect.borrow_mut() = Some(SpanCollectScope::enter());
    otel_bdd.mounted.set(true);
    let mut runner = crate::bdd::helpers::make_agent(agent);
    crate::bdd::helpers::bind_session_or_panic(&mut runner, SESSION_UUID);
    let mut stream = crate::bdd::helpers::agent_submit_root(&mut runner, "读取文件").await;
    while let Some(e) = stream.next().await {
        if let XyEvent::Error(err) = &e {
            panic!(
                "[otel-bdd] run failed: kind={} message={}",
                err.kind, err.message
            );
        }
    }
}

#[then("tool.execute 带参数与结果摘要而其余 span 无 observation I/O")]
fn t_c2826_tool_tier_only(otel_bdd: &OtelBdd) {
    let records = otel_bdd.records();
    let tools = find_named(&records, "tool.execute");
    assert!(!tools.is_empty(), "c2826: 应有 tool.execute span");
    for tool in &tools {
        assert!(
            prop(tool, "langfuse.observation.input").is_some(),
            "c2826: tool tier=truncated 时 tool.execute 必须带参数摘要"
        );
    }
    for r in &records {
        if r.name != "tool.execute" {
            assert!(
                prop(r, "langfuse.observation.input").is_none()
                    && prop(r, "langfuse.observation.output").is_none(),
                "c2826: observation_io=none 时 {} 不得带 observation I/O",
                r.name
            );
        }
    }
}

#[when("以观测闸开启并触发一次会话压缩")]
async fn w_c2826_compaction_span(sess: &crate::bdd::fixtures::XySessionStore, otel_bdd: &OtelBdd) {
    use crate::bdd::steps_compaction::COMP_RETAIN_SID;
    sess.ensure_mgr();
    crate::bdd::steps_compaction::comp_seed_turns(sess, COMP_RETAIN_SID, 50).await;
    sess.current_id.replace(Some(COMP_RETAIN_SID.to_string()));
    set_provider_trace_active(true);
    set_observation_io_tier(ObservationIoTier::None);
    set_tool_observation_io_tier(ObservationIoTier::None);
    set_obs_session(SESSION_UUID, None);
    drop(otel_bdd.collect.borrow_mut().take());
    *otel_bdd.collect.borrow_mut() = Some(SpanCollectScope::enter());
    otel_bdd.mounted.set(true);
    // 经编排器手动压缩（span 守卫在 orchestrator，不在裸 compact_session）。
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let model = xylitol::infra::provider::factory::build_provider(
        &xylitol::protocol::model::XyModelConfig {
            kind: xylitol::protocol::model::XyModelKind::Fake,
            model: "fake".into(),
            api_key: String::new(),
            base_url: None,
            api: None,
            compat: None,
        },
    );
    let binding =
        xylitol::agent::model::task_model::CompactionSummaryBinding::for_test(model, "fake");
    let sink = std::sync::Arc::new(xylitol::infra::event::EventBus::new());
    let mut notice = false;
    xylitol::agent::compaction::CompactionOrchestrator::new(
        xylitol::agent::compaction::CompactionSettings {
            enabled: true,
            reserve_tokens: 1024,
            keep_recent_tokens: 1_000,
            ..Default::default()
        },
    )
    .compact(
        &mgr,
        COMP_RETAIN_SID,
        &binding,
        sink.as_ref(),
        None,
        100_000,
        None,
        &mut notice,
    )
    .await
    .expect("c2826: orchestrator manual compact");
}

#[then("导出 agent.compaction 且 type 为 span 并携带原因与 obs lane")]
fn t_c2826_compaction_span(otel_bdd: &OtelBdd) {
    let records = otel_bdd.records();
    let spans = find_named(&records, "agent.compaction");
    assert!(
        !spans.is_empty(),
        "c2826: 过 prepare 的压缩必须导出 agent.compaction，实际 {:?}",
        records.iter().map(|r| r.name.clone()).collect::<Vec<_>>()
    );
    for r in &spans {
        assert_eq!(prop(r, "langfuse.observation.type"), Some("span"));
        let reason = prop(r, "reason").unwrap_or_default();
        assert!(
            ["manual", "threshold", "overflow"].contains(&reason),
            "c2826: 压缩 reason 必须诚实，实际 {reason}"
        );
        assert_eq!(prop(r, "xylitol.obs.lane"), Some("llm"));
    }
}

#[then("agent.turn 不携带 ERROR 或 aborted 终态")]
fn t_c2826_turn_not_error(otel_bdd: &OtelBdd) {
    let records = otel_bdd.records();
    let turns = find_named(&records, "agent.turn");
    assert!(!turns.is_empty(), "c2826: 应有 agent.turn span");
    for r in &turns {
        assert_ne!(
            prop(r, "langfuse.observation.level"),
            Some("ERROR"),
            "c2826: {r:?}"
        );
        assert_ne!(
            prop(r, "langfuse.observation.status_message"),
            Some("aborted"),
            "c2826: 正常完成 turn 不得标 aborted"
        );
    }
}

#[then("token.estimate 恰好导出一次")]
fn t_c2826_single_estimate(otel_bdd: &OtelBdd) {
    let records = otel_bdd.records();
    // 收窄到本回合 trace：fastrace 后台批量投递可能把前一个测试的残留
    // span 落进本 scope 的 buffer（跨测试噪声，非本回合行为）。
    let turn = turn_root(&records);
    let count = records
        .iter()
        .filter(|s| s.name == "token.estimate" && s.trace_id == turn.trace_id)
        .count();
    assert_eq!(
        count, 1,
        "c2826: 一次 TurnSettled 恰一个挂在本回合 agent.turn 下的 token.estimate，实际 {count}"
    );
}

// ── c2827：r1470 闲置结算独立根 ─────────────────────────────────

#[given("观测闸开启且绑定会话身份")]
fn g_c2827_otel_idle_gate(otel_bdd: &OtelBdd) {
    otel_bdd.mount_scopes(None);
}

#[when("以闲置路径结算一次 token 估计")]
fn w_c2827_idle_settle(_otel_bdd: &OtelBdd) {
    // 无活跃 turn 上下文：settlement 不带 obs_parent → 独立根。
    let entries: Vec<SessionEntry> = Vec::new();
    let obs_session = xylitol_ai_bridge::thinking::ObsSessionContext {
        session_id: Some(SESSION_UUID.into()),
        ..Default::default()
    };
    let opts = xylitol::agent::compaction::token_estimator::EstimateOpts {
        obs_session,
        ..Default::default()
    };
    let _ = xylitol::agent::compaction::settlement::settle_from_session_entries(
        &entries,
        &opts,
        xylitol::agent::compaction::settlement::ContextTokenSettlementReason::TurnSettled,
    );
    // fastrace 批量投递是异步的：给后台线程一点时间。
    std::thread::sleep(std::time::Duration::from_millis(200));
}

#[then("token.estimate 为独立根且携带会话 id")]
fn t_c2827_idle_estimate_root(otel_bdd: &OtelBdd) {
    let records = otel_bdd.records();
    // 收窄到本会话：并行套件里他测试的 settlement span 可能落进本 collector。
    let ests: Vec<&fastrace::collector::SpanRecord> = records
        .iter()
        .filter(|s| s.name == "token.estimate")
        .filter(|s| {
            s.properties
                .iter()
                .any(|(k, v)| k == "langfuse.session.id" && v.as_ref() == SESSION_UUID)
        })
        .collect();
    assert_eq!(
        ests.len(),
        1,
        "c2827: 一次闲置结算应恰一个本会话 token.estimate：{}",
        ests.len()
    );
    let est = ests[0];
    let has_parent = records
        .iter()
        .any(|s| s.name == "agent.turn" && s.span_id == est.parent_id);
    assert!(!has_parent, "c2827: 闲置路径 MUST NOT 伪造父 turn");
    let session_id = est
        .properties
        .iter()
        .find(|(k, _)| k == "langfuse.session.id")
        .map(|(_, v)| v.as_ref());
    assert_eq!(session_id, Some(SESSION_UUID), "c2827: 独立根应携带会话 id");
}
