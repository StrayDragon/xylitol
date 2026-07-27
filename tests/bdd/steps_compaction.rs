use crate::fixtures::*;
use crate::helpers::*;
use crate::prelude::*;
use rstest_bdd_macros::{given, then, when};

mod comp_fixture {
    use std::cell::RefCell;
    use xylitol::infra::session::SessionEntry;
    thread_local! {
        pub static BRANCH_SKIPPED: RefCell<Vec<SessionEntry>> = const { RefCell::new(Vec::new()) };
        pub static LAST_COMPACTION: RefCell<Option<xylitol::infra::session::CompactionEntry>> =
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
    use xylitol::agent::compaction::{CompactionSettings, compact_session};

    reset_fake_state();
    set_fake_text(
        "## Goal\nRetain recent context\n\n## Progress\n### Done\n- [x] summarized\n\n## Next Steps\n1. Continue\n",
    );
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let model = xylitol::infra::provider::factory::build_provider(
        &xylitol::protocol::model_config::XyModelConfig {
            kind: xylitol::protocol::model_config::XyModelKind::Fake,
            model: "fake".into(),
            api_key: String::new(),
            base_url: None,
            api: None,
        },
    )
    .expect("build fake provider");
    let settings = CompactionSettings {
        enabled: true,
        reserve_tokens: 1024,
        keep_recent_tokens,
    };
    let result = compact_session(&mgr, sid, model.as_ref(), &settings).await;
    agent.last_result.replace(Some(
        result
            .as_ref()
            .map(|e| format!("compacted:{}", e.summary.len()))
            .map_err(|e| XyDriverError::from(e.clone())),
    ));
    if let Ok(entry) = result {
        comp_fixture::LAST_COMPACTION.with(|c| c.replace(Some(entry.clone())));
        let entries = mgr.load(sid).await.unwrap_or_default();
        sess.entries.replace(entries);
        sess.current_id.replace(Some(sid.to_string()));
    }
}

pub(crate) fn compaction_entry_from_sess(
    sess: &XySessionStore,
) -> xylitol::infra::session::CompactionEntry {
    if let Some(e) = comp_fixture::LAST_COMPACTION.with(|c| c.borrow().clone()) {
        return e;
    }
    let entry = sess
        .entries
        .borrow()
        .iter()
        .find_map(|e| match e {
            SessionEntry::Compaction(c) => Some(c.clone()),
            _ => None,
        })
        .expect("expected CompactionEntry in session");
    entry
}

/// Active context after compaction: one CompactionEntry plus message turns from `firstKeptEntryId`.
pub(crate) fn comp_active_record_counts(entries: &[SessionEntry]) -> (usize, usize) {
    let compaction = entries.iter().find_map(|e| match e {
        SessionEntry::Compaction(c) => Some(c.clone()),
        _ => None,
    });
    let Some(comp) = compaction else {
        let msgs = entries
            .iter()
            .filter(|e| matches!(e, SessionEntry::Message(_)))
            .count();
        return (msgs, msgs);
    };
    let keep_from = entries
        .iter()
        .position(|e| e.entry_id() == Some(comp.first_kept_entry_id.as_str()))
        .unwrap_or(entries.len());
    let kept_messages = entries[keep_from..]
        .iter()
        .filter(|e| matches!(e, SessionEntry::Message(_)))
        .count();
    (1 + kept_messages, kept_messages)
}

#[given("配置了上下文窗口为 100000 的模型")]
pub(crate) fn _g_comp_config_window(agent: &AgentState) {
    agent.context_window.set(100_000);
}

#[given("会话消息估算使用 {tokens:u32} 个 token")]
pub(crate) fn _g_comp_tokens(agent: &AgentState, tokens: u32) {
    agent
        .last_result
        .replace(Some(Ok(format!("tokens:{tokens}"))));
}

#[given("压缩阈值为 {val:f64}")]
pub(crate) fn _g_comp_threshold(agent: &AgentState, val: f64) {
    agent.compaction_threshold.set(val);
}

#[when("调用 shouldCompact")]
pub(crate) fn _w_comp_check(agent: &AgentState) {
    let tokens: u64 = agent
        .last_result
        .borrow()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .and_then(|s| s.strip_prefix("tokens:").and_then(|n| n.parse().ok()))
        .unwrap_or(0);
    let window = agent.context_window.get().max(1);
    agent.compaction_result.replace(Some(should_compact(
        tokens,
        window,
        agent.compaction_threshold.get(),
    )));
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
    use xylitol::protocol::session::ForkPosition;

    let sid = "branch-bound-parent";
    comp_seed_turns(sess, sid, 12).await;
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let assistant = SessionEntry::Message(MessageEntry {
        base: EntryBase {
            entry_type: "message".into(),
            id: "flush-asst".into(),
            parent_id: None,
            timestamp: "2024-01-01T00:00:01Z".into(),
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
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let _ = mgr.create(sid, Some("."), None).await;
    for i in 0..turns {
        let e = SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: format!("msg-{i}"),
                parent_id: None,
                timestamp: "2024-01-01T00:00:00Z".into(),
            },
            message: serde_json::json!({
                "role": "user",
                "content": format!("turn {i} {}", "x".repeat(400)),
            }),
        });
        let _ = mgr.append(sid, &e).await;
    }
}
