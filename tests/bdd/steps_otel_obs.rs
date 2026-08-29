//! Steps for `infra-otel` — 低频观测 span 族（otel6/7/8/10/11/12/22）。
//!
//! 经真实 AgentRuntime（fake provider + 工具调用）在观测闸 + 收集槽下驱动；
//! 断言落在收集到的 fastrace `SpanRecord` 属性上。otel18–22 由单测覆盖
//! （spec 明文禁 BDD step），otel1–5 / 9 / 13 / 17 / 20 / 21 维持单测与
//! live smoke 承载，不在本文件范围。

use crate::fixtures::AgentState;
use crate::prelude::*;
use rstest::fixture;
use rstest_bdd_macros::{then, when};
use xylitol::infra::provider::factory::{set_fake_text, set_fake_tool_call, set_fake_tool_result};
use xylitol_ai_bridge::provider::trace::{
    ObservationIoTier, SpanCollectScope, set_observation_io_tier, set_provider_trace_active,
    set_tool_observation_io_tier,
};
use xylitol_ai_bridge::provider::{clear_obs_session, set_obs_session};

const SESSION_UUID: &str = "aaagggg-hhhh-iiii-jjjj-kkkkllllmmmm";

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

    fn records(&self) -> Vec<fastrace::collector::SpanRecord> {
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
async fn run_turn_with_tool(agent: &AgentState, session_name: Option<&str>) {
    set_fake_text("我来读文件");
    set_fake_tool_call("read", r#"{"path":"src/main.rs"}"#);
    set_fake_tool_result("hello world");
    let mut runner = crate::helpers::make_agent(agent);
    // 显式绑定已知会话 UUID——runtime bind_session 会把 obs session 槽
    // 同步为当前会话，otel6 的「当前会话 UUID」即此 id。
    crate::helpers::bind_session_or_panic(&mut runner, SESSION_UUID);
    if let Some(name) = session_name {
        xylitol_ai_bridge::provider::set_obs_session_name(Some(name));
    }
    let mut stream = crate::helpers::agent_submit_root(&mut runner, "读取文件").await;
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
