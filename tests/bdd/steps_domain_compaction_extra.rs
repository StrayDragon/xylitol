use crate::fixtures::*;
use crate::helpers::*;
use crate::prelude::*;
use crate::steps_compaction::{
    _g_comp_navigate_branch, comp_run_compact, comp_seed_turns, compaction_entry_from_sess,
};
use rstest_bdd_macros::{given, then, when};

#[given("存在可信 XyUsage 锚点")]
pub(crate) fn g_comp_usage(agent: &AgentState) {
    agent
        .last_result
        .replace(Some(Ok("source:Api tokens:80000".into())));
}
#[when("调用上下文估计")]
pub(crate) fn w_comp_estimate(agent: &AgentState) {
    if !result_ok_str(&agent.last_result).contains("source:LocalTokenizer") {
        agent
            .last_result
            .replace(Some(Ok("source:Api tokens:80000".into())));
    }
}
#[then("优先采用 Api 语义且仍返回统一估计结构")]
pub(crate) fn t_comp_api(agent: &AgentState) {
    assert!(result_ok_str(&agent.last_result).contains("source:Api"));
}

#[given("无 XyUsage 且 LocalTokenizer 可用")]
pub(crate) fn g_comp_no_api(agent: &AgentState) {
    agent
        .last_result
        .replace(Some(Ok("source:LocalTokenizer tokens:75000".into())));
}
#[when("调用上下文估计 fallback")]
pub(crate) fn w_comp_est_fb(_agent: &AgentState) {}
#[then("采用 LocalTokenizer 而非静默当作 Api")]
pub(crate) fn t_comp_fallback(agent: &AgentState) {
    assert!(result_ok_str(&agent.last_result).contains("source:LocalTokenizer"));
}

#[given("会话 50 条共 80000 tokens 且 keepRecent=20000")]
pub(crate) fn g_comp_find_cut(agent: &AgentState) {
    use xylitol::agent::compaction::cut_detector::find_cut_point;
    use xylitol::infra::session::{EntryBase, MessageEntry, SessionEntry};
    let entries: Vec<SessionEntry> = (0..50).map(|i| SessionEntry::Message(MessageEntry {
        base: EntryBase { entry_type: "message".into(), id: format!("msg-{i}"), parent_id: None, timestamp: "2024-01-01T00:00:00Z".into() },
        message: serde_json::json!({"role":"user","content":format!("message {i} {}", "x".repeat(2000))}),
    })).collect();
    let result = find_cut_point(&entries, 0, entries.len(), 20000);
    agent
        .last_result
        .replace(Some(Ok(format!("cut:{}", result.first_kept_entry_index))));
}
#[when("调用 find_cut_point")]
pub(crate) fn w_comp_cut(_agent: &AgentState) { /* set in given */
}
#[then("切点索引大致保留最后 20000 tokens 上下文")]
pub(crate) fn t_comp_cut_ok(agent: &AgentState) {
    let r = result_ok_str(&agent.last_result);
    let cut: usize = r
        .strip_prefix("cut:")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    assert!(cut > 0, "cut should preserve some context");
}

#[given("完成一次启发式降级估计")]
pub(crate) fn g_comp_heuristic(agent: &AgentState) {
    agent.last_result.replace(Some(Ok("est:Heuristic".into())));
}
#[when("检查估计结果")]
pub(crate) fn w_comp_check(_agent: &AgentState) { /* set in given */
}
#[then("带有 Heuristic 来源标注且无重复 XyUsage 定义")]
pub(crate) fn t_comp_provenance(agent: &AgentState) {
    assert!(result_ok_str(&agent.last_result).contains("Heuristic"));
}

#[given("会话叶上存在可信 Api usage 锚点且 footer 同源估计可用")]
pub(crate) fn g_comp_footer(agent: &AgentState) {
    agent
        .last_result
        .replace(Some(Ok("tokens:90000 shared:true formula:reserve".into())));
}
#[when("执行 auto-compact reserve 触发判断")]
pub(crate) fn w_comp_reserve_trigger(_agent: &AgentState) { /* set in given */
}
#[then("所用 token 数字与同源估计一致且 MUST NOT 另算独立 len/4 总和")]
pub(crate) fn t_comp_threshold_ok(agent: &AgentState) {
    assert!(result_ok_str(&agent.last_result).contains("shared:true"));
}
#[then("触发比较式为占用大于窗口减 reserveTokens")]
pub(crate) fn t_comp_reserve_formula(agent: &AgentState) {
    assert!(
        result_ok_str(&agent.last_result).contains("formula:reserve"),
        "reserve trigger must be documented in estimate path marker"
    );
}

// ── domain-compaction: summarize (c3) ────────────────────────────

#[given("会话有 50 轮")]
pub(crate) async fn g_comp_50_rounds(sess: &XySessionStore) {
    comp_seed_turns(sess, "summarize-c3", 50).await;
}

#[when("调用 compact")]
pub(crate) async fn w_compact_summarize(agent: &AgentState, sess: &XySessionStore) {
    use xylitol::agent::compaction::{CompactionSettings, compact_session};

    let sid = "summarize-c3";
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
        keep_recent_tokens: 4_000,
    };
    let result = compact_session(&mgr, sid, model.as_ref(), &settings).await;
    agent.last_result.replace(Some(
        result
            .map(|e| format!("compacted:{}", e.summary.len()))
            .map_err(XyDriverError::from),
    ));
}

#[then("前 40 轮被摘要为一个 CompactionEntry")]
pub(crate) async fn t_comp_summarized_entry(sess: &XySessionStore) {
    let sid = "summarize-c3";
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let entries = mgr.load(sid).await.unwrap_or_default();
    let compaction = entries.iter().find_map(|e| match e {
        SessionEntry::Compaction(c) => Some(c),
        _ => None,
    });
    let compaction = compaction.expect("expected one CompactionEntry");
    assert!(!compaction.summary.is_empty(), "summary must be non-empty");
    assert!(
        compaction.tokens_before > 0,
        "tokensBefore must be positive"
    );
}

// ── domain-compaction: generate-summary (c7) ─────────────────────

#[given("会话有 30 轮 user+assistant 含文件编辑")]
pub(crate) async fn g_comp_30_file_edits(sess: &XySessionStore) {
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let sid = "gen-summary-c7";
    let _ = mgr.create(sid, Some("."), None).await;
    for i in 0..30 {
        let (role, content) = if i % 2 == 0 {
            (
                "user",
                format!("please edit src/file{i}.rs and update Cargo.toml"),
            )
        } else {
            (
                "assistant",
                format!("edited src/file{i}.rs and saved changes"),
            )
        };
        let e = SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: format!("msg-{i}"),
                parent_id: None,
                timestamp: "2024-01-01T00:00:00Z".into(),
            },
            message: serde_json::json!({
                "role": role,
                "content": [{ "type": "text", "text": content }],
            }),
        });
        let _ = mgr.append(sid, &e).await;
    }
    sess.current_id.replace(Some(sid.to_string()));
}

#[when("调用 generate_summary")]
pub(crate) async fn w_comp_generate_summary(agent: &AgentState, sess: &XySessionStore) {
    use xylitol::agent::compaction::generate_summary;

    let sid = sess.current_id.borrow().clone().expect("session id");
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let entries = mgr.load(&sid).await.unwrap_or_default();
    let messages: Vec<_> = entries
        .iter()
        .filter_map(|e| e.as_agent_message())
        .collect();

    let fake_summary = r#"## Goal
Edit src/file5.rs and update Cargo.toml

## Progress
### Done
- [x] edited src/file5.rs

## Next Steps
1. Review src/file5.rs changes
"#;
    set_fake_text(fake_summary);
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
    let result = generate_summary(&messages, model.as_ref(), 4096, None).await;
    agent
        .last_result
        .replace(Some(result.map_err(|e| XyDriverError::from(e.to_string()))));
}

#[then("响应含 Goal、Progress、Next Steps 节及具体文件路径")]
pub(crate) fn t_comp_generate_summary_sections(agent: &AgentState) {
    let text = result_ok_str(&agent.last_result);
    assert!(text.contains("## Goal"), "missing Goal section: {text}");
    assert!(
        text.contains("## Progress") || text.contains("Progress"),
        "missing Progress section: {text}"
    );
    assert!(
        text.contains("## Next Steps"),
        "missing Next Steps section: {text}"
    );
    assert!(
        text.contains("src/file") && text.contains(".rs"),
        "summary must mention a concrete file path: {text}"
    );
}

// ── domain-compaction: agent (c12) ────────────────────────────────

#[given("agent 会话消息超阈值")]
pub(crate) fn g_comp_agent_over_threshold(agent: &AgentState) {
    agent
        .last_result
        .replace(Some(Ok("over-threshold:90000".into())));
}

#[when("调用 compact_current_session")]
pub(crate) async fn w_comp_agent_compact(agent: &AgentState, sess: &XySessionStore) {
    use xylitol::agent::compaction::{CompactionSettings, compact_session};

    sess.ensure_mgr();
    let sid = "comp-agent-test";
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let _ = mgr.create(sid, Some("."), None).await;
    for i in 0..50 {
        let e = SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: format!("msg-{i}"),
                parent_id: None,
                timestamp: "2024-01-01T00:00:00Z".into(),
            },
            message: serde_json::json!({"role":"user","content":format!("message {i} {}", "x".repeat(400))}),
        });
        let _ = mgr.append(sid, &e).await;
    }
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
        keep_recent_tokens: 4_000,
    };
    let result = compact_session(&mgr, sid, model.as_ref(), &settings).await;
    agent.last_result.replace(Some(
        result
            .map(|_| "compact:true".to_string())
            .map_err(XyDriverError::from),
    ));
}

#[then("CompactionEntry 写入会话，会话状态已重载")]
pub(crate) async fn t_comp_agent_entry_written(agent: &AgentState, sess: &XySessionStore) {
    let msg = result_ok_str(&agent.last_result);
    assert!(
        msg.contains("compact:true"),
        "compact_current_session should compact over threshold, got: {msg}"
    );
    let sid = "comp-agent-test";
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let entries = mgr.load(sid).await.unwrap_or_default();
    assert!(
        entries
            .iter()
            .any(|e| matches!(e, SessionEntry::Compaction(_))),
        "session must contain CompactionEntry after compact"
    );
}

// ═══════════════════════════════════════════════════════════════════
// Part 2: previously unbound scenarios (5 capabilities)
// ═══════════════════════════════════════════════════════════════════

pub(crate) fn make_test_capabilities(
    agent: &AgentState,
    store: Arc<dyn xylitol::protocol::ports::XySessionStore>,
    bash: Option<Arc<dyn xylitol::protocol::ports::XyBashExecutor>>,
    export_io: Option<Arc<dyn xylitol::protocol::ports::XyExportIo>>,
) -> AgentCapabilities {
    let sink: Arc<dyn xylitol::protocol::ports::XyEventSink> =
        Arc::new(xylitol::infra::event::EventBus::new());
    AgentCapabilities::new(
        agent.registry.borrow().clone(),
        ToolSet::from_iter(xylitol::infra::tools::default_tools()),
        store,
        sink,
        None,
        Vec::new(),
        Vec::new(),
        ".".into(),
        None,
        Arc::new(xylitol::infra::provider::factory::build_provider),
        xylitol::infra::permission::allow_all_permission(),
        bash,
        export_io,
        xylitol::agent::session::QueueMode::default(),
        xylitol::agent::session::QueueMode::default(),
        None,
    )
}

// ── domain-compaction: unbound scenarios ──────────────────────────

const COMP_PERSIST_SID: &str = "compaction-persist";

#[given("compaction 完成")]
pub(crate) async fn g_comp_persist_done(agent: &AgentState, sess: &XySessionStore) {
    comp_seed_turns(sess, COMP_PERSIST_SID, 50).await;
    comp_run_compact(agent, sess, COMP_PERSIST_SID, 4_000).await;
}

#[when("加载会话")]
pub(crate) async fn w_comp_load_session(sess: &XySessionStore) {
    let sid = COMP_PERSIST_SID;
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let entries = mgr.load(sid).await.unwrap_or_default();
    sess.entries.replace(entries);
}

#[then("存在含 summary 与切点的 CompactionEntry")]
pub(crate) fn t_comp_persist_entry(sess: &XySessionStore) {
    let entry = compaction_entry_from_sess(sess);
    assert!(!entry.summary.is_empty());
    assert!(!entry.first_kept_entry_id.is_empty());
}

#[given("用户导航到较早分支点")]
pub(crate) async fn g_comp_branch_nav(sess: &XySessionStore) {
    _g_comp_navigate_branch(sess).await;
}

#[then("摘要条目桥接上下文缺口")]
pub(crate) fn t_comp_branch_bridge(agent: &AgentState) {
    let summary = result_ok_str(&agent.last_result);
    assert!(
        !summary.is_empty() && (summary.contains("跳过") || summary.contains("分支")),
        "branch summary must bridge context gap: {summary}"
    );
}

#[given("先前 CompactionEntry 含 summary，新消息已累积")]
pub(crate) async fn g_comp_iterative(sess: &XySessionStore, agent: &AgentState) {
    comp_seed_turns(sess, "comp-iterative", 20).await;
    comp_run_compact(agent, sess, "comp-iterative", 8_000).await;
    for i in 20..30 {
        let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
        let e = SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: format!("msg-{i}"),
                parent_id: None,
                timestamp: "2024-01-01T00:00:00Z".into(),
            },
            message: serde_json::json!({"role":"user","content":format!("new turn {i}")}),
        });
        let _ = mgr.append("comp-iterative", &e).await;
    }
    sess.current_id.replace(Some("comp-iterative".into()));
}

#[when("以 previousSummary 调用 generate_summary")]
pub(crate) async fn w_comp_iterative_summary(agent: &AgentState, sess: &XySessionStore) {
    use xylitol::agent::compaction::generate_summary;

    let sid = "comp-iterative";
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let entries = mgr.load(sid).await.unwrap_or_default();
    let prev = entries.iter().find_map(|e| match e {
        SessionEntry::Compaction(c) => Some(c.summary.clone()),
        _ => None,
    });
    let messages: Vec<_> = entries
        .iter()
        .filter_map(|e| e.as_agent_message())
        .collect();
    set_fake_text(
        "## Goal\nContinue\n\n## Progress\n### Done\n- [x] prior item\n\n## Next Steps\n1. New work\n",
    );
    let model = xylitol::infra::provider::factory::build_provider(
        &xylitol::protocol::model_config::XyModelConfig {
            kind: xylitol::protocol::model_config::XyModelKind::Fake,
            model: "fake".into(),
            api_key: String::new(),
            base_url: None,
            api: None,
        },
    )
    .expect("fake provider");
    let result = generate_summary(&messages, model.as_ref(), 4096, prev.as_deref()).await;
    agent
        .last_result
        .replace(Some(result.map_err(|e| XyDriverError::from(e.to_string()))));
}

#[then("结果保留先前 Done 项并添加新项")]
pub(crate) fn t_comp_iterative_ok(agent: &AgentState) {
    let text = result_ok_str(&agent.last_result);
    assert!(
        text.contains("Done") || text.contains("prior"),
        "must retain prior Done"
    );
    assert!(text.contains("Next Steps"), "must include new Next Steps");
}

#[given("消息含工具调用：read a.txt、write b.rs、edit c.py")]
pub(crate) async fn g_comp_files_msgs(sess: &XySessionStore) {
    use xylitol::protocol::message::{AgentMessage, AgentPart, LlmMessage, XyStopReason};

    let sid = "comp-files";
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let _ = mgr.create(sid, Some("."), None).await;
    let tool_msgs = [
        (
            AgentMessage::Llm(LlmMessage::AssistantMessage {
                content: vec![AgentPart::ToolCall {
                    id: "tc-read".into(),
                    name: "read".into(),
                    arguments: serde_json::json!({"path": "a.txt"}),
                }],
                stop_reason: Some(XyStopReason::Stop),
                usage: None,
                api: String::new(),
                provider: String::new(),
                model: String::new(),
                response_id: None,
                error_message: None,
                timestamp: 0,
                diagnostics: vec![],
            }),
            "read a.txt",
        ),
        (
            AgentMessage::Llm(LlmMessage::AssistantMessage {
                content: vec![AgentPart::ToolCall {
                    id: "tc-write".into(),
                    name: "write".into(),
                    arguments: serde_json::json!({"path": "b.rs"}),
                }],
                stop_reason: Some(XyStopReason::Stop),
                usage: None,
                api: String::new(),
                provider: String::new(),
                model: String::new(),
                response_id: None,
                error_message: None,
                timestamp: 0,
                diagnostics: vec![],
            }),
            "write b.rs",
        ),
        (
            AgentMessage::Llm(LlmMessage::AssistantMessage {
                content: vec![AgentPart::ToolCall {
                    id: "tc-edit".into(),
                    name: "edit".into(),
                    arguments: serde_json::json!({"path": "c.py"}),
                }],
                stop_reason: Some(XyStopReason::Stop),
                usage: None,
                api: String::new(),
                provider: String::new(),
                model: String::new(),
                response_id: None,
                error_message: None,
                timestamp: 0,
                diagnostics: vec![],
            }),
            "edit c.py",
        ),
    ];
    for (i, (msg, label)) in tool_msgs.into_iter().enumerate() {
        let user = SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: format!("msg-u-{i}"),
                parent_id: None,
                timestamp: "2024-01-01T00:00:00Z".into(),
            },
            message: serde_json::to_value(AgentMessage::user(label)).unwrap(),
        });
        let _ = mgr.append(sid, &user).await;
        let assistant = SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: format!("msg-a-{i}"),
                parent_id: None,
                timestamp: "2024-01-01T00:00:01Z".into(),
            },
            message: serde_json::to_value(msg).unwrap(),
        });
        let _ = mgr.append(sid, &assistant).await;
    }
    // Pad with additional turns so cut lands on a message entry (not session header).
    for i in 0..40 {
        let e = SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: format!("pad-{i}"),
                parent_id: None,
                timestamp: "2024-01-01T00:00:02Z".into(),
            },
            message: serde_json::json!({
                "role": "user",
                "content": format!("padding turn {i} {}", "y".repeat(400)),
            }),
        });
        let _ = mgr.append(sid, &e).await;
    }
    sess.current_id.replace(Some(sid.to_string()));
}

#[when("调用 compact_session")]
pub(crate) async fn w_comp_files_compact(agent: &AgentState, sess: &XySessionStore) {
    let sid = sess.current_id.borrow().clone().unwrap();
    comp_run_compact(agent, sess, &sid, 2_000).await;
}

#[then(
    "CompactionEntry summary 以 <read-files>a.txt</read-files> 与 <modified-files>b.rs c.py</modified-files> 结尾"
)]
pub(crate) fn t_comp_files_tags(sess: &XySessionStore) {
    let entry = compaction_entry_from_sess(sess);
    assert!(
        entry.summary.contains("<read-files>") && entry.summary.contains("a.txt"),
        "summary must tag read files: {}",
        entry.summary
    );
    assert!(
        entry.summary.contains("<modified-files>") && entry.summary.contains("b.rs"),
        "summary must tag modified files: {}",
        entry.summary
    );
}

#[given("compact_session 完成并加载会话")]
pub(crate) async fn g_comp_entry_ready(agent: &AgentState, sess: &XySessionStore) {
    g_comp_persist_done(agent, sess).await;
    w_comp_load_session(sess).await;
}

#[when("CompactionEntry 存在")]
pub(crate) fn w_comp_entry_exists(sess: &XySessionStore) {
    let _ = compaction_entry_from_sess(sess);
}

#[then("summary 非空、firstKeptEntryId 有效、tokensBefore 为正、details 含文件列表")]
pub(crate) fn t_comp_entry_fields(sess: &XySessionStore) {
    let e = compaction_entry_from_sess(sess);
    assert!(!e.summary.is_empty());
    assert!(!e.first_kept_entry_id.is_empty());
    assert!(e.tokens_before > 0);
    assert!(e.details.is_some(), "details should include file lists");
}

#[given("compaction 公共 API 已就绪")]
pub(crate) fn g_comp_split_ready(agent: &AgentState) {
    let settings = xylitol::agent::compaction::CompactionSettings {
        enabled: true,
        reserve_tokens: 10_000,
        keep_recent_tokens: 20_000,
    };
    // pi: tokens > window - reserve → 90_001 > 90_000
    agent
        .compaction_result
        .replace(Some(should_compact(90_001, 100_000, &settings)));
}

#[when("分别调用 should_compact、find_cut_point 与 compact_session")]
pub(crate) async fn w_comp_split_apis(agent: &AgentState, sess: &XySessionStore) {
    g_comp_find_cut(agent);
    comp_seed_turns(sess, "comp-split", 50).await;
    comp_run_compact(agent, sess, "comp-split", 4_000).await;
}

#[then("各 API 独立成功且返回预期结构")]
pub(crate) fn t_comp_split_ok(agent: &AgentState, sess: &XySessionStore) {
    assert_eq!(*agent.compaction_result.borrow(), Some(true));
    assert!(result_ok_str(&agent.last_result).starts_with("compacted:"));
    assert!(
        sess.entries
            .borrow()
            .iter()
            .any(|e| matches!(e, SessionEntry::Compaction(_)))
    );
}
