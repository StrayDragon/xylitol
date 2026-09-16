use crate::tests::bdd::fixtures::*;
use crate::tests::bdd::helpers::*;
use crate::tests::bdd::prelude::*;
use rstest_bdd_macros::{given, then, when};

mod comp_fixture {
    use crate::infra::session::SessionEntry;
    use std::cell::RefCell;
    thread_local! {
        pub static BRANCH_SKIPPED: RefCell<Vec<SessionEntry>> = const { RefCell::new(Vec::new()) };
        pub static LAST_COMPACTION: RefCell<Option<crate::infra::session::CompactionEntry>> =
            const { RefCell::new(None) };
    }
}

const COMP_RETAIN_SID: &str = "compaction-retain";
const COMP_WRITE_SID: &str = "compaction-write";

pub(crate) async fn comp_run_compact(
    agent: &AgentState,
    sess: &XySessionStore,
    sid: &str,
    keep_recent_tokens: u64,
) {
    use crate::agent::compaction::{CompactionSettings, compact_session};

    reset_fake_state();
    set_fake_text(
        "## Goal\nRetain recent context\n\n## Progress\n### Done\n- [x] summarized\n\n## Next Steps\n1. Continue\n",
    );
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let model =
        crate::infra::provider::factory::build_provider(&crate::protocol::model::XyModelConfig {
            kind: crate::protocol::model::XyModelKind::Fake,
            model: "fake".into(),
            api_key: String::new(),
            base_url: None,
            api: None,
            compat: None,
        });
    let settings = CompactionSettings {
        enabled: true,
        reserve_tokens: 1024,
        keep_recent_tokens,
        ..Default::default()
    };
    let result = compact_session(
        &mgr,
        sid,
        &crate::agent::model::task_model::CompactionSummaryBinding::for_test(model, "fake"),
        &settings,
        None,
        0,
        None,
        None,
    )
    .await;
    match result {
        Ok(entry) => {
            agent
                .last_result
                .replace(Some(Ok(format!("compacted:{}", entry.summary.len()))));
            comp_fixture::LAST_COMPACTION.with(|c| c.replace(Some(entry.clone())));
            let entries = mgr.load(sid).await.unwrap_or_default();
            sess.entries.replace(entries);
            sess.current_id.replace(Some(sid.to_string()));
        }
        Err(e) => {
            agent.last_result.replace(Some(Err(XyDriverError::from(e))));
        }
    }
}

pub(crate) fn compaction_entry_from_sess(
    sess: &XySessionStore,
) -> crate::infra::session::CompactionEntry {
    if let Some(e) = comp_fixture::LAST_COMPACTION.with(|c| c.borrow().clone()) {
        return e;
    }
    sess.entries
        .borrow()
        .iter()
        .find_map(|e| match e {
            SessionEntry::Compaction(c) => Some(c.clone()),
            _ => None,
        })
        .expect("expected CompactionEntry in session")
}

/// Active context after compaction: one CompactionEntry plus message turns from `firstKeptEntryId`.
///
/// `session_env` bootstrap rows (c1906 post-compact ensure) count toward total active
/// records but **not** toward the "轮" (turn) count in retain-recent assertions.
pub(crate) fn comp_active_record_counts(entries: &[SessionEntry]) -> (usize, usize) {
    use crate::protocol::session::build_context_entries;
    let ctx = build_context_entries(entries);
    let mut msgs = 0usize;
    let mut turn_msgs = 0usize;
    for e in &ctx {
        if !matches!(e, SessionEntry::Message(_)) {
            continue;
        }
        msgs += 1;
        let is_session_env = e
            .as_agent_message()
            .is_some_and(|m| crate::agent::prompt::session_env_from_message(&m).is_some());
        if !is_session_env {
            turn_msgs += 1;
        }
    }
    let has_compaction = ctx.iter().any(|e| matches!(e, SessionEntry::Compaction(_)));
    if has_compaction {
        (1 + msgs, turn_msgs)
    } else {
        (msgs, turn_msgs)
    }
}

#[given("配置了上下文窗口为 {n:u64} 的模型")]
pub(crate) fn _g_comp_config_window_n(agent: &AgentState, n: u64) {
    agent.context_window.set(n);
}

#[given("会话消息估算使用 {tokens:u32} 个 token")]
pub(crate) fn _g_comp_tokens(agent: &AgentState, tokens: u32) {
    agent
        .last_result
        .replace(Some(Ok(format!("tokens:{tokens}"))));
}

#[given("compaction reserveTokens 为 {reserve:u64}")]
pub(crate) fn _g_comp_reserve(agent: &AgentState, reserve: u64) {
    agent.compaction_reserve_tokens.set(reserve);
}

#[given("compaction enabled 为 true")]
pub(crate) fn _g_comp_enabled_true(agent: &AgentState) {
    agent.compaction_enabled.set(true);
}

#[given("compaction enabled 为 false")]
pub(crate) fn _g_comp_enabled_false(agent: &AgentState) {
    agent.compaction_enabled.set(false);
}

#[when("调用 shouldCompact")]
pub(crate) fn _w_comp_check(agent: &AgentState) {
    use crate::agent::compaction::{
        projected_post_compact_tokens, should_compact, summary_placeholder_tokens,
    };
    let tokens: u64 = agent
        .last_result
        .borrow()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .and_then(|s| s.strip_prefix("tokens:").and_then(|n| n.parse().ok()))
        .unwrap_or(0);
    let window = agent.context_window.get();
    let settings = crate::agent::compaction::CompactionSettings {
        enabled: agent.compaction_enabled.get(),
        reserve_tokens: agent.compaction_reserve_tokens.get(),
        keep_recent_tokens: agent.compaction_keep_tokens.get(),
        ..Default::default()
    };
    // c2: floor-aware threshold — overhead defaults to 0 (degenerate reserve formula)
    // unless a 固定请求开销 given step injected one.
    let floor = projected_post_compact_tokens(
        &settings,
        window,
        agent.compaction_fixed_overhead.get(),
        summary_placeholder_tokens(&[]),
    );
    let should = should_compact(tokens, window, &settings, floor);
    agent.compaction_result.replace(Some(should));
}

#[given("compaction keepRecentTokens 为 {n:u64}")]
pub(crate) fn _g_comp_keep(agent: &AgentState, n: u64) {
    agent.compaction_keep_tokens.set(n);
}

#[given("compaction 固定请求开销为 {n:u64} token")]
pub(crate) fn _g_comp_overhead(agent: &AgentState, n: u64) {
    agent.compaction_fixed_overhead.set(n);
}

#[then("返回 true")]
pub(crate) fn _t_comp_result_true(agent: &AgentState) {
    assert_eq!(*agent.compaction_result.borrow(), Some(true));
}
#[then("返回 false")]
pub(crate) fn _t_comp_result_false(agent: &AgentState) {
    assert_eq!(*agent.compaction_result.borrow(), Some(false));
}

#[given("会话有 50 个轮次")]
pub(crate) async fn _g_comp_50_turns(sess: &XySessionStore) {
    comp_seed_turns(sess, COMP_RETAIN_SID, 50).await;
    sess.current_id.replace(Some(COMP_RETAIN_SID.to_string()));
}

#[when("触发压缩保留最近 10 轮")]
pub(crate) async fn _w_comp_trigger(agent: &AgentState, sess: &XySessionStore) {
    comp_run_compact(agent, sess, COMP_RETAIN_SID, 1_000).await;
}

#[then("前 40 轮被总结为一个 CompactionEntry")]
pub(crate) fn _t_comp_has_summary(sess: &XySessionStore) {
    let entry = compaction_entry_from_sess(sess);
    assert!(
        !entry.summary.is_empty(),
        "CompactionEntry summary must be non-empty"
    );
}

#[then("会话中剩余 {n:u32} 条记录（概要 + {m:u32} 轮）")]
pub(crate) fn _t_comp_remaining(sess: &XySessionStore, n: u32, m: u32) {
    let entries = sess.entries.borrow();
    let (count, msg_count) = comp_active_record_counts(&entries);
    assert_eq!(
        count, n as usize,
        "expected {n} active records (summary + {m} turns), got {count}"
    );
    let compaction_count = entries
        .iter()
        .filter(|e| matches!(e, SessionEntry::Compaction(_)))
        .count();
    assert_eq!(compaction_count, 1, "expected exactly one CompactionEntry");
    assert_eq!(
        msg_count, m as usize,
        "expected {m} message turns in active context, got {msg_count}"
    );
}

#[given("会话正在活跃使用")]
pub(crate) async fn _g_comp_active(sess: &XySessionStore) {
    comp_seed_turns(sess, COMP_WRITE_SID, 50).await;
    sess.current_id.replace(Some(COMP_WRITE_SID.to_string()));
}

#[when("压缩完成")]
pub(crate) async fn _w_comp_done(agent: &AgentState, sess: &XySessionStore) {
    comp_run_compact(agent, sess, COMP_WRITE_SID, 4_000).await;
}

#[then("会话 JSONL 包含 CompactionEntry")]
pub(crate) fn _t_comp_jsonl_has_entry(sess: &XySessionStore) {
    assert!(
        sess.entries
            .borrow()
            .iter()
            .any(|e| matches!(e, SessionEntry::Compaction(_))),
        "session must contain CompactionEntry"
    );
}

#[then("CompactionEntry 包含 summary 字段")]
pub(crate) fn _t_comp_has_summary_field(sess: &XySessionStore) {
    let e = compaction_entry_from_sess(sess);
    assert!(!e.summary.is_empty(), "summary field must be non-empty");
}

#[then("CompactionEntry 包含 firstKeptEntryId 字段")]
pub(crate) fn _t_comp_has_firstkept(sess: &XySessionStore) {
    let e = compaction_entry_from_sess(sess);
    assert!(
        !e.first_kept_entry_id.is_empty(),
        "firstKeptEntryId must be set"
    );
}

#[then("CompactionEntry 包含 tokensBefore 字段")]
pub(crate) fn _t_comp_has_tokensbefore(sess: &XySessionStore) {
    let e = compaction_entry_from_sess(sess);
    assert!(e.tokens_before > 0, "tokensBefore must be positive");
}

#[given("用户在树中导航到分支点")]
pub(crate) async fn _g_comp_navigate_branch(sess: &XySessionStore) {
    use crate::protocol::session::ForkPosition;

    let sid = "branch-bound-parent";
    comp_seed_turns(sess, sid, 12).await;
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let assistant = SessionEntry::Message(MessageEntry {
        base: EntryBase {
            entry_type: "message".into(),
            id: "flush-asst".into(),
            parent_id: None,
            timestamp: 1704067201000,
        },
        message: serde_json::json!({"role":"assistant","content":"ok"}),
    });
    let _ = mgr.append(sid, &assistant).await;
    let entries = mgr.load(sid).await.unwrap_or_default();
    let skipped: Vec<SessionEntry> = entries
        .iter()
        .filter(|e| matches!(e, SessionEntry::Message(_)))
        .take(6)
        .cloned()
        .collect();
    comp_fixture::BRANCH_SKIPPED.with(|s| s.replace(skipped));
    let _ = mgr
        .fork(sid, "branch-bound-child", "msg-5", ForkPosition::Before)
        .await;
    sess.current_id
        .replace(Some("branch-bound-child".to_string()));
}

#[when("生成分支摘要")]
pub(crate) fn _w_comp_branch_summary(agent: &AgentState, sess: &XySessionStore) {
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let skipped = comp_fixture::BRANCH_SKIPPED.with(|s| s.borrow().clone());
    let summary = mgr.generate_branch_summary(&skipped);
    agent.last_result.replace(Some(Ok(summary)));
}

#[then("摘要描述了被跳过的上下文")]
pub(crate) fn _t_comp_branch_desc(agent: &AgentState) {
    let summary = result_ok_str(&agent.last_result);
    assert!(
        summary.contains("跳过") || summary.contains("条记录"),
        "branch summary must describe skipped context, got: {summary}"
    );
}

#[then("当前上下文是连贯的")]
pub(crate) fn _t_comp_context_coherent(agent: &AgentState, sess: &XySessionStore) {
    let summary = result_ok_str(&agent.last_result);
    assert!(!summary.is_empty(), "branch summary must be non-empty");
    assert!(
        sess.current_id.borrow().is_some(),
        "active session must remain set after branch navigation"
    );
}

pub(crate) async fn comp_seed_turns(sess: &XySessionStore, sid: &str, turns: usize) {
    use crate::protocol::message::AgentMessage;
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let _ = mgr.create(sid, Some("."), None).await;
    for i in 0..turns {
        let e = SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: format!("msg-{i}"),
                parent_id: None,
                timestamp: 1704067200000,
            },
            message: serde_json::to_value(AgentMessage::user(format!(
                "turn {i} {}",
                "x".repeat(400)
            )))
            .unwrap(),
        });
        let _ = mgr.append(sid, &e).await;
    }
}
