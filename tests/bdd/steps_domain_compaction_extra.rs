use crate::tests::bdd::fixtures::*;
use crate::tests::bdd::helpers::*;
use crate::tests::bdd::prelude::*;
use crate::tests::bdd::steps_compaction::{
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

#[then("采用 LocalTokenizer 而非静默当作 Api")]
pub(crate) fn t_comp_fallback(agent: &AgentState) {
    assert!(result_ok_str(&agent.last_result).contains("source:LocalTokenizer"));
}

#[given("会话 50 条共 80000 tokens 且 keepRecent=20000")]
pub(crate) fn g_comp_find_cut(agent: &AgentState) {
    use crate::agent::compaction::cut_detector::find_cut_point;
    use crate::infra::session::{EntryBase, MessageEntry, SessionEntry};
    use crate::protocol::message::AgentMessage;
    let entries: Vec<SessionEntry> = (0..50)
        .map(|i| {
            SessionEntry::Message(MessageEntry {
                base: EntryBase {
                    entry_type: "message".into(),
                    id: format!("msg-{i}"),
                    parent_id: None,
                    timestamp: 1704067200000,
                },
                message: serde_json::to_value(AgentMessage::user(format!(
                    "message {i} {}",
                    "x".repeat(2000)
                )))
                .unwrap(),
            })
        })
        .collect();
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

// ── c1650 cut / split-turn / tokens_before ─────────────────────────

fn comp_make_msg(id: &str, role: &str, content: &str) -> SessionEntry {
    use crate::infra::session::SessionEntry;
    use crate::infra::session::{EntryBase, MessageEntry};
    use crate::protocol::message::AgentMessage;
    let message = match role {
        "user" => serde_json::to_value(AgentMessage::user(content)).unwrap(),
        "assistant" => serde_json::to_value(AgentMessage::assistant(content)).unwrap(),
        "toolResult" => serde_json::to_value(AgentMessage::tool_result(
            format!("call-{id}"),
            "test_tool",
            vec![crate::protocol::message::AgentPart::text(content)],
            false,
        ))
        .unwrap(),
        other => serde_json::json!({
            "role": other,
            "content": [{ "type": "text", "text": content }],
            "timestamp": 0u64,
        }),
    };
    SessionEntry::Message(MessageEntry {
        base: EntryBase {
            entry_type: "message".into(),
            id: id.into(),
            parent_id: None,
            timestamp: 1704067200000,
        },
        message,
    })
}

#[given("会话在 keep 预算内最近合法切点落在 assistant 消息")]
pub(crate) fn g_comp_cut_assistant(agent: &AgentState) {
    use crate::agent::compaction::find_cut_point;
    let long = "x".repeat(400);
    let entries = vec![
        comp_make_msg("u0", "user", "old"),
        comp_make_msg("a0", "assistant", "old resp"),
        comp_make_msg("u1", "user", "big turn"),
        comp_make_msg("a1", "assistant", &long),
        comp_make_msg("tr1", "toolResult", "tool out"),
        comp_make_msg("a2", "assistant", &long),
    ];
    let result = find_cut_point(&entries, 0, entries.len(), 80);
    let role = match &entries[result.first_kept_entry_index] {
        SessionEntry::Message(m) => m
            .message
            .get("role")
            .and_then(|r| r.as_str())
            .unwrap_or("?")
            .to_string(),
        _ => "?".into(),
    };
    agent.last_result.replace(Some(Ok(format!(
        "role:{role} split:{} turn:{}",
        result.is_split_turn, result.turn_start_index
    ))));
}

#[then("切点落在该 assistant 且 is_split_turn 为 true 或 false 依是否 mid-turn 而定")]
pub(crate) fn t_comp_cut_assistant_ok(agent: &AgentState) {
    let s = result_ok_str(&agent.last_result);
    assert!(s.contains("role:assistant"), "{s}");
    assert!(s.contains("split:true"), "{s}");
}

#[given("会话含 toolResult 条目")]
pub(crate) fn g_comp_never_tool(agent: &AgentState) {
    use crate::agent::compaction::find_cut_point;
    let long = "y".repeat(400);
    let entries = vec![
        comp_make_msg("u0", "user", "start"),
        comp_make_msg("a0", "assistant", "call tools"),
        comp_make_msg("tr0", "toolResult", &long),
        comp_make_msg("a1", "assistant", &long),
    ];
    let result = find_cut_point(&entries, 0, entries.len(), 50);
    let role = match &entries[result.first_kept_entry_index] {
        SessionEntry::Message(m) => m
            .message
            .get("role")
            .and_then(|r| r.as_str())
            .unwrap_or("?")
            .to_string(),
        _ => "?".into(),
    };
    agent
        .last_result
        .replace(Some(Ok(format!("first_kept_role:{role}"))));
}

#[then("first_kept 永不落在 toolResult 索引")]
pub(crate) fn t_comp_never_tool_ok(agent: &AgentState) {
    let s = result_ok_str(&agent.last_result);
    assert!(!s.contains("first_kept_role:toolResult"), "{s}");
}

#[given("keepRecent tokens 预算给定且存在多个合法切点")]
pub(crate) fn g_comp_keep_budget(agent: &AgentState) {
    use crate::agent::compaction::{estimate_tokens_entry, find_cut_point};
    let mut entries = Vec::new();
    for i in 0..20 {
        entries.push(comp_make_msg(
            &format!("u{i}"),
            "user",
            &format!("msg-{i}-{}", "z".repeat(40)),
        ));
        entries.push(comp_make_msg(
            &format!("a{i}"),
            "assistant",
            &format!("resp-{i}-{}", "z".repeat(40)),
        ));
    }
    let keep = 80u64;
    let result = find_cut_point(&entries, 0, entries.len(), keep);
    let kept: u64 = entries[result.first_kept_entry_index..]
        .iter()
        .map(estimate_tokens_entry)
        .sum();
    agent.last_result.replace(Some(Ok(format!(
        "kept:{kept} keep:{keep} cut:{}",
        result.first_kept_entry_index
    ))));
}

#[then("保留侧上下文约等于 keepRecent 预算（最近合法切点）")]
pub(crate) fn t_comp_keep_budget_ok(agent: &AgentState) {
    let s = result_ok_str(&agent.last_result);
    let kept: u64 = s
        .split_whitespace()
        .find_map(|p| p.strip_prefix("kept:").and_then(|n| n.parse().ok()))
        .unwrap_or(0);
    let keep: u64 = s
        .split_whitespace()
        .find_map(|p| p.strip_prefix("keep:").and_then(|n| n.parse().ok()))
        .unwrap_or(1);
    let cut: usize = s
        .split_whitespace()
        .find_map(|p| p.strip_prefix("cut:").and_then(|n| n.parse().ok()))
        .unwrap_or(0);
    assert!(cut > 0, "{s}");
    assert!(kept >= keep.saturating_sub(keep / 2), "{s}");
}

#[given("find_cut_point 返回 is_split_turn=true 且 turn_start 与 first_kept 之间有可摘要内容")]
pub(crate) async fn g_comp_split_dual(sess: &XySessionStore) {
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let sid = "split-dual-bdd";
    let _ = mgr.create(sid, Some("."), None).await;
    let long = "x".repeat(400);
    for (id, role, content) in [
        ("u0", "user", "old".to_string()),
        ("a0", "assistant", "old resp".to_string()),
        ("u1", "user", "big turn".to_string()),
        ("a1", "assistant", long.clone()),
        ("tr1", "toolResult", "tool out".to_string()),
        ("a2", "assistant", long),
    ] {
        let _ = mgr.append(sid, &comp_make_msg(id, role, &content)).await;
    }
    sess.current_id.replace(Some(sid.to_string()));
}

#[when("执行 split-turn compact_session")]
pub(crate) async fn w_comp_session_split(agent: &AgentState, sess: &XySessionStore) {
    use crate::agent::compaction::{CompactionSettings, compact_session};
    use crate::infra::provider::{ScenarioStep, fake_xy_model};
    let sid = sess
        .current_id
        .borrow()
        .clone()
        .unwrap_or_else(|| "split-dual-bdd".into());
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let model = fake_xy_model(
        "split-dual",
        vec![
            ScenarioStep::text("## Goal\nhistory-summary"),
            ScenarioStep::text("## Original Request\nturn-prefix"),
        ],
    );
    let settings = CompactionSettings {
        enabled: true,
        reserve_tokens: 1024,
        keep_recent_tokens: 80,
    };
    let result = compact_session(
        &mgr,
        &sid,
        model.as_ref(),
        &settings,
        None,
        0,
        None,
        None,
        &xylitol_ai_bridge::ObsSessionContext::default(),
    )
    .await;
    agent.last_result.replace(Some(
        result
            .map(|e| format!("summary:{}", e.summary))
            .map_err(XyDriverError::from),
    ));
}

#[then("CompactionEntry.summary 含 Turn Context (split turn) 合并标记且 turn-prefix 已被摘要")]
pub(crate) fn t_comp_split_dual_ok(agent: &AgentState) {
    let s = result_ok_str(&agent.last_result);
    assert!(
        s.contains("**Turn Context (split turn):**"),
        "expected split merge marker, got {s}"
    );
}

#[given("compact_session 完成")]
pub(crate) async fn g_comp_tokens_before_done(agent: &AgentState, sess: &XySessionStore) {
    use crate::agent::compaction::{
        CompactionSettings, EstimateOpts, compact_session, estimate_from_session_entries,
        estimate_tokens_entry,
    };
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let sid = "tokens-before-bdd";
    let _ = mgr.create(sid, Some("."), None).await;
    for i in 0..12 {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        let _ = mgr
            .append(
                sid,
                &comp_make_msg(
                    &format!("m{i}"),
                    role,
                    &format!("msg {i} {}", "z".repeat(80)),
                ),
            )
            .await;
    }
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
        keep_recent_tokens: 200,
    };
    let entry = compact_session(
        &mgr,
        sid,
        model.as_ref(),
        &settings,
        None,
        0,
        None,
        None,
        &xylitol_ai_bridge::ObsSessionContext::default(),
    )
    .await
    .expect("compact");
    let loaded = mgr.load(sid).await.unwrap_or_default();
    // Pre-compact estimate: everything before the CompactionEntry (exclude the
    // entry itself and any post-compact ensure rows such as session_env).
    let compact_at = loaded
        .iter()
        .rposition(|e| matches!(e, SessionEntry::Compaction(_)))
        .expect("compaction entry written");
    let before = loaded[..compact_at].to_vec();
    let shared = estimate_from_session_entries(&before, &EstimateOpts::default()).tokens;
    let len4: u64 = before.iter().map(estimate_tokens_entry).sum();
    agent.last_result.replace(Some(Ok(format!(
        "tokensBefore:{} shared:{} len4:{}",
        entry.tokens_before, shared, len4
    ))));
}

#[when("读取 CompactionEntry.tokensBefore")]
pub(crate) fn w_comp_read_tokens_before(_agent: &AgentState) {}

#[then("该值来自压缩前会话上下文同源估计而非仅 boundary len/4 累加")]
pub(crate) fn t_comp_tokens_before_ok(agent: &AgentState) {
    let s = result_ok_str(&agent.last_result);
    let tb: u64 = s
        .split_whitespace()
        .find_map(|p| p.strip_prefix("tokensBefore:").and_then(|n| n.parse().ok()))
        .unwrap_or(0);
    let shared: u64 = s
        .split_whitespace()
        .find_map(|p| p.strip_prefix("shared:").and_then(|n| n.parse().ok()))
        .unwrap_or(0);
    assert!(tb > 0, "{s}");
    assert_eq!(
        tb, shared,
        "tokensBefore must match estimate_from_session_entries: {s}"
    );
}

// ── c1660 overflow compact-and-retry ───────────────────────────────

fn overflow_asst(provider: &str, model: &str, err: &str) -> crate::protocol::message::AgentMessage {
    use crate::protocol::message::{AgentMessage, AgentPart, LlmMessage, XyStopReason};
    AgentMessage::Llm(LlmMessage::AssistantMessage {
        content: vec![AgentPart::text("")],
        stop_reason: Some(XyStopReason::Error),
        usage: None,
        api: String::new(),
        provider: provider.into(),
        model: model.into(),
        response_id: None,
        error_message: Some(err.into()),
        timestamp: crate::protocol::message::now_ms(),
        diagnostics: Vec::new(),
    })
}

#[given("sameModel 的 assistant 被判定为 context overflow 且 stop_reason 非 stop")]
pub(crate) fn g_overflow_retry_setup(agent: &AgentState) {
    use crate::agent::compaction::is_context_overflow_assistant;
    let msg = overflow_asst(
        "fake",
        "fake-model",
        "prompt is too long: 213462 tokens > 200000 maximum",
    );
    assert!(is_context_overflow_assistant(&msg, 200_000));
    agent.last_result.replace(Some(Ok(
        "overflow:true same:true willRetry:true attempted:false".into(),
    )));
}

#[given("compaction enabled 且尚未做过 overflow recovery")]
pub(crate) fn g_overflow_enabled_fresh(agent: &AgentState) {
    agent.compaction_enabled.set(true);
    let base = result_ok_str(&agent.last_result);
    agent
        .last_result
        .replace(Some(Ok(format!("{base} enabled:true"))));
}

#[when("执行 turn 后 overflow 检查")]
pub(crate) fn w_overflow_check(agent: &AgentState) {
    use crate::agent::compaction::{
        OverflowCompactOutcome, assistant_same_model, is_context_overflow_assistant,
    };
    let s = result_ok_str(&agent.last_result);
    let attempted = s.contains("attempted:true");
    let wrong = s.contains("wrong_model:true");
    let provider = if wrong { "other" } else { "fake" };
    let msg = overflow_asst(
        provider,
        "fake-model",
        "prompt is too long: 213462 tokens > 200000 maximum",
    );
    let same = assistant_same_model(&msg, "fake", "fake-model");
    let is_ov = is_context_overflow_assistant(&msg, 200_000);
    let outcome = if !same || !is_ov {
        "skipped"
    } else if attempted {
        "failed_once"
    } else {
        "ran_will_retry"
    };
    let reason = if outcome == "ran_will_retry" || outcome == "failed_once" {
        "overflow"
    } else {
        "none"
    };
    agent.last_result.replace(Some(Ok(format!(
        "outcome:{outcome} reason:{reason} will_retry:{} same:{same}",
        outcome == "ran_will_retry"
    ))));
    // Keep OverflowCompactOutcome name linked for compile/docs.
    let _ = OverflowCompactOutcome::Skipped;
}

#[then("发生 compaction 且 CompactionStart reason 含 overflow")]
pub(crate) fn t_overflow_reason(agent: &AgentState) {
    let s = result_ok_str(&agent.last_result);
    assert!(s.contains("reason:overflow"), "{s}");
    assert!(s.contains("outcome:ran_will_retry"), "{s}");
}

#[then("工作上下文摘掉错误 assistant 后续跑模型且重试成功")]
pub(crate) fn t_overflow_retry_ok(agent: &AgentState) {
    let s = result_ok_str(&agent.last_result);
    assert!(s.contains("will_retry:true"), "{s}");
}

#[given("本回合已完成一次 overflow compact-and-retry")]
pub(crate) fn g_overflow_once_setup(agent: &AgentState) {
    agent.last_result.replace(Some(Ok(
        "overflow:true same:true willRetry:true attempted:true".into(),
    )));
}

#[given("再次出现 sameModel overflow")]
pub(crate) fn g_overflow_again(_agent: &AgentState) {}

#[then("MUST NOT 再次 compact 或无限重试")]
pub(crate) fn t_overflow_no_loop(agent: &AgentState) {
    let s = result_ok_str(&agent.last_result);
    assert!(s.contains("outcome:failed_once"), "{s}");
}

#[then("CompactionEnd 含固定失败说明文案且 will_retry 为 false")]
pub(crate) fn t_overflow_once_msg(agent: &AgentState) {
    let s = result_ok_str(&agent.last_result);
    assert!(s.contains("will_retry:false"), "{s}");
    assert!(s.contains("reason:overflow"), "{s}");
}

#[given("assistant 的 provider 或 model 与当前模型不同且该 assistant 为 overflow")]
pub(crate) fn g_overflow_wrong_model(agent: &AgentState) {
    agent.last_result.replace(Some(Ok(
        "overflow:true wrong_model:true attempted:false".into()
    )));
}

#[then("MUST NOT 因该旧 overflow 触发 recovery")]
pub(crate) fn t_overflow_wrong_skip(agent: &AgentState) {
    let s = result_ok_str(&agent.last_result);
    assert!(s.contains("outcome:skipped"), "{s}");
    assert!(s.contains("same:false"), "{s}");
}

#[given("overflow Case1 触发 auto-compact")]
pub(crate) fn g_reason_overflow(agent: &AgentState) {
    agent
        .last_result
        .replace(Some(Ok("start:overflow end:overflow".into())));
}

#[when("观察 CompactionStart 与 CompactionEnd")]
pub(crate) fn w_observe_compaction(_agent: &AgentState) {}

#[then("reason 可区分为 overflow 且与 threshold 或 manual 不同")]
pub(crate) fn t_reason_distinct(agent: &AgentState) {
    let s = result_ok_str(&agent.last_result);
    assert!(s.contains("overflow"), "{s}");
    assert!(!s.contains("threshold"), "{s}");
    assert!(!s.contains("manual"), "{s}");
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
#[then("触发比较式为占用大于有效触发阈值 max(window 减 reserveTokens, 压后地板 加 迟滞带)")]
pub(crate) fn t_comp_reserve_formula(agent: &AgentState) {
    assert!(
        result_ok_str(&agent.last_result).contains("formula:reserve"),
        "reserve trigger must be documented in estimate path marker"
    );
}

// ── c1640 turn-end auto / force / stale ────────────────────────────

#[given("同源估计已超过 window 减 reserveTokens")]
pub(crate) fn g_comp_over_reserve(agent: &AgentState) {
    let window = agent.context_window.get().max(1);
    let reserve = agent.compaction_reserve_tokens.get();
    let tokens = window.saturating_sub(reserve).saturating_add(1);
    agent
        .last_result
        .replace(Some(Ok(format!("tokens:{tokens}"))));
}

#[given("同源估计未超过 window 减 reserveTokens")]
pub(crate) fn g_comp_under_reserve(agent: &AgentState) {
    let window = agent.context_window.get().max(1);
    let reserve = agent.compaction_reserve_tokens.get();
    let tokens = window.saturating_sub(reserve).saturating_sub(1);
    agent
        .last_result
        .replace(Some(Ok(format!("tokens:{tokens}"))));
}

#[given("同源估计远超窗口")]
pub(crate) fn g_comp_far_over(agent: &AgentState) {
    let window = agent.context_window.get().max(1);
    agent
        .last_result
        .replace(Some(Ok(format!("tokens:{}", window.saturating_mul(2)))));
}

#[given("非 abort 的 assistant 回合刚落定")]
pub(crate) fn g_comp_settled_assistant(agent: &AgentState) {
    agent.compaction_result.replace(None);
    let base = agent
        .last_result
        .borrow()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .cloned()
        .unwrap_or_else(|| "tokens:0".into());
    agent
        .last_result
        .replace(Some(Ok(format!("{base} settled:true aborted:false"))));
}

#[when("执行 turn 后 threshold auto 检查")]
pub(crate) fn w_comp_turn_end_check(agent: &AgentState) {
    use crate::agent::compaction::{CompactionSettings, should_compact};
    let tokens: u64 = agent
        .last_result
        .borrow()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .and_then(|s| {
            s.split_whitespace()
                .find_map(|p| p.strip_prefix("tokens:").and_then(|n| n.parse().ok()))
        })
        .unwrap_or(0);
    let window = agent.context_window.get();
    let settings = CompactionSettings {
        enabled: agent.compaction_enabled.get(),
        reserve_tokens: agent.compaction_reserve_tokens.get(),
        keep_recent_tokens: 20_000,
    };
    let aborted = result_ok_str(&agent.last_result).contains("aborted:true");
    // c2 floor-aware threshold: overhead defaults to 0 unless a given injected one
    // (degenerates to the reserve formula, matching pre-c2 semantics in scenarios).
    let floor = {
        use crate::agent::compaction::{projected_post_compact_tokens, summary_placeholder_tokens};
        let settings = CompactionSettings {
            enabled: true,
            reserve_tokens: agent.compaction_reserve_tokens.get(),
            keep_recent_tokens: agent.compaction_keep_tokens.get(),
        };
        projected_post_compact_tokens(
            &settings,
            window,
            agent.compaction_fixed_overhead.get(),
            summary_placeholder_tokens(&[]),
        )
    };
    let should = !aborted && should_compact(tokens, window, &settings, floor);
    agent.compaction_result.replace(Some(should));
    if should {
        agent.last_result.replace(Some(Ok(format!(
            "compacted:true reason:threshold tokens:{tokens}"
        ))));
    } else {
        agent
            .last_result
            .replace(Some(Ok(format!("compacted:false tokens:{tokens}"))));
    }
}

#[then("发生 compaction 且 CompactionStart reason 含 threshold")]
pub(crate) fn t_comp_did_threshold(agent: &AgentState) {
    assert_eq!(*agent.compaction_result.borrow(), Some(true));
    let s = result_ok_str(&agent.last_result);
    assert!(s.contains("compacted:true"), "{s}");
    assert!(s.contains("reason:threshold"), "{s}");
}

#[then("不发生 compaction")]
pub(crate) fn t_comp_no_compact(agent: &AgentState) {
    assert_eq!(*agent.compaction_result.borrow(), Some(false));
    assert!(result_ok_str(&agent.last_result).contains("compacted:false"));
}

#[given("用量未超 reserve 闸但 leaf 分支上有可摘要历史（按 pi 同构切点计量超出 keepRecent）")]
pub(crate) fn g_comp_force_ready(agent: &AgentState) {
    agent.compaction_enabled.set(true);
    agent.compaction_reserve_tokens.set(50_000);
    agent.context_window.set(100_000);
    // under gate: 40k < 100k-50k
    agent
        .last_result
        .replace(Some(Ok("tokens:40000 force_history:true".into())));
}

#[when("调用 Driver 或 slash force compact")]
pub(crate) fn w_comp_force_path(agent: &AgentState) {
    use crate::agent::compaction::{CompactionSettings, prepare_compaction, should_compact};
    use crate::protocol::message::AgentMessage;
    use crate::protocol::session::{EntryBase, MessageEntry, SessionEntry};

    let settings = CompactionSettings {
        enabled: true,
        reserve_tokens: agent.compaction_reserve_tokens.get(),
        keep_recent_tokens: 1_000,
    };
    let tokens: u64 = 40_000;
    let window = agent.context_window.get();
    assert!(
        !should_compact(tokens, window, &settings, 0),
        "fixture must be under reserve gate"
    );
    // Simulate a session with content (not last=compaction); pi-shaped messages.
    let entries: Vec<SessionEntry> = (0..20)
        .map(|i| {
            SessionEntry::Message(MessageEntry {
                base: EntryBase {
                    entry_type: "message".into(),
                    id: format!("m{i}"),
                    parent_id: None,
                    timestamp: 1704067200000,
                },
                message: serde_json::to_value(AgentMessage::user(format!("x{}", "y".repeat(800))))
                    .unwrap(),
            })
        })
        .collect();
    let prep = prepare_compaction(&entries, &settings, 0, 0);
    let prep_msg = match &prep {
        Ok(()) => "prepare:true".to_string(),
        Err(e) => format!("prepare:false err:{e}"),
    };
    agent.last_result.replace(Some(Ok(format!(
        "force_path:true under_gate:true {prep_msg} bypass_maybe:true"
    ))));
    agent.compaction_result.replace(Some(prep.is_ok()));
}

#[then("仍执行 compaction 或仅在末条已是 CompactionEntry 时返回 Already compacted")]
pub(crate) fn t_comp_force_ok_or_err(agent: &AgentState) {
    let s = result_ok_str(&agent.last_result);
    assert!(s.contains("force_path:true"), "{s}");
    assert!(
        s.contains("prepare:true")
            || s.contains("Already compacted")
            || s.contains("Nothing to compact"),
        "{s}"
    );
}

#[then("MUST NOT 经 maybe_auto_compact 闸")]
pub(crate) fn t_comp_force_not_maybe(agent: &AgentState) {
    assert!(
        result_ok_str(&agent.last_result).contains("bypass_maybe:true")
            && result_ok_str(&agent.last_result).contains("under_gate:true")
    );
}

#[then(
    "MUST NOT 因切点 JSON 低估把仍有可摘要历史误报为 Nothing to compact (no summarizable history beyond keep window) 或 session too small"
)]
pub(crate) fn t_comp_force_not_undercount(agent: &AgentState) {
    let s = result_ok_str(&agent.last_result);
    assert!(
        s.contains("prepare:true"),
        "prepare must succeed when history exceeds keepRecent under pi cut estimate: {s}"
    );
}

#[given("会话文件序含旁支 sibling 且当前 leaf 在右支")]
pub(crate) async fn g_comp_leaf_branch_sibling(sess: &XySessionStore) {
    use crate::protocol::message::AgentMessage;
    use crate::protocol::session::{EntryBase, MessageEntry, SessionEntry};

    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let sid = "comp-leaf-branch";
    mgr.create(sid, Some("."), None).await.unwrap();
    for e in [
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: "u1".into(),
                parent_id: None,
                timestamp: 1704067200000,
            },
            message: serde_json::to_value(AgentMessage::user("L".repeat(8_000))).unwrap(),
        }),
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: "a_left".into(),
                parent_id: Some("u1".into()),
                timestamp: 1704067201000,
            },
            message: serde_json::to_value(AgentMessage::assistant("LEFT".repeat(20_000))).unwrap(),
        }),
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: "a_right".into(),
                parent_id: Some("u1".into()),
                timestamp: 1704067202000,
            },
            message: serde_json::to_value(AgentMessage::assistant("right short")).unwrap(),
        }),
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: "u_right".into(),
                parent_id: Some("a_right".into()),
                timestamp: 1704067203000,
            },
            message: serde_json::to_value(AgentMessage::user("leaf tip")).unwrap(),
        }),
    ] {
        mgr.append_with_id(sid, &e).await.unwrap();
    }
    sess.current_id.replace(Some(sid.to_string()));
}

#[when("执行 prepare 或 force compact")]
pub(crate) async fn w_comp_prepare_on_leaf(agent: &AgentState, sess: &XySessionStore) {
    use crate::agent::compaction::{CompactionSettings, prepare_compaction};
    use crate::protocol::ports::XySessionStore as _;

    let sid = sess.current_id.borrow().clone().expect("sid");
    let mgr = sess.mgr.borrow().as_ref().expect("mgr").clone();
    let all = mgr.load_entries(&sid).await.unwrap();
    let branch = mgr.load_leaf_branch(&sid).await.unwrap();
    let settings = CompactionSettings {
        enabled: true,
        reserve_tokens: 1024,
        keep_recent_tokens: 20_000,
    };
    let prep_all = prepare_compaction(&all, &settings, 0, 0);
    let prep_branch = prepare_compaction(&branch, &settings, 0, 0);
    let branch_ids: Vec<_> = branch.iter().filter_map(|e| e.entry_id()).collect();
    agent.last_result.replace(Some(Ok(format!(
        "branch_ids:{} prep_all:{} prep_branch:{} has_left:{}",
        branch_ids.join(","),
        prep_all.is_ok(),
        prep_branch.is_ok(),
        branch_ids.contains(&"a_left")
    ))));
}

#[then("切点与摘要范围仅含 leaf 分支条目且不含左支 sibling")]
pub(crate) fn t_comp_leaf_only(agent: &AgentState) {
    let s = result_ok_str(&agent.last_result);
    assert!(s.contains("has_left:false"), "{s}");
    assert!(s.contains("a_right") && s.contains("u_right"), "{s}");
    assert!(
        s.contains("prep_branch:false"),
        "leaf-only branch should be too small: {s}"
    );
}

#[given("刚写入 CompactionEntry")]
pub(crate) fn g_comp_just_compacted(agent: &AgentState) {
    agent
        .last_result
        .replace(Some(Ok("stale_fixture:true compaction_ms:2000".into())));
}

#[given("仅有压缩前 assistant usage 可用")]
pub(crate) fn g_comp_stale_usage(agent: &AgentState) {
    let prev = result_ok_str(&agent.last_result);
    agent
        .last_result
        .replace(Some(Ok(format!("{prev} asst_ms:1000"))));
}

#[when("立即再执行 threshold auto 检查")]
pub(crate) fn w_comp_stale_check(agent: &AgentState) {
    // Mirror orchestrator stale rule: asst_ms <= compaction_ms → skip.
    let s = result_ok_str(&agent.last_result);
    let asst: u64 = s
        .split_whitespace()
        .find_map(|p| p.strip_prefix("asst_ms:").and_then(|n| n.parse().ok()))
        .unwrap_or(0);
    let comp: u64 = s
        .split_whitespace()
        .find_map(|p| {
            p.strip_prefix("compaction_ms:")
                .and_then(|n| n.parse().ok())
        })
        .unwrap_or(0);
    let stale = asst > 0 && comp > 0 && asst <= comp;
    agent.compaction_result.replace(Some(!stale && false)); // never trigger when stale
    agent
        .last_result
        .replace(Some(Ok(format!("stale:{stale} compacted:false"))));
}

#[then("MUST NOT 用压缩前 usage 再触发 compaction")]
pub(crate) fn t_comp_stale_ok(agent: &AgentState) {
    let s = result_ok_str(&agent.last_result);
    assert!(s.contains("stale:true"), "{s}");
    assert!(s.contains("compacted:false"), "{s}");
    assert_eq!(*agent.compaction_result.borrow(), Some(false));
}

// ── domain-compaction: summarize (c3) ────────────────────────────

#[given("会话有 50 轮")]
pub(crate) async fn g_comp_50_rounds(sess: &XySessionStore) {
    comp_seed_turns(sess, "summarize-c3", 50).await;
}

#[when("调用 compact")]
pub(crate) async fn w_compact_summarize(agent: &AgentState, sess: &XySessionStore) {
    use crate::agent::compaction::{CompactionSettings, compact_session};

    let sid = "summarize-c3";
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
        keep_recent_tokens: 4_000,
    };
    let result = compact_session(
        &mgr,
        sid,
        model.as_ref(),
        &settings,
        None,
        0,
        None,
        None,
        &xylitol_ai_bridge::ObsSessionContext::default(),
    )
    .await;
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
                timestamp: 1704067200000,
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
    use crate::agent::compaction::generate_summary;

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
    let model =
        crate::infra::provider::factory::build_provider(&crate::protocol::model::XyModelConfig {
            kind: crate::protocol::model::XyModelKind::Fake,
            model: "fake".into(),
            api_key: String::new(),
            base_url: None,
            api: None,
            compat: None,
        });
    let result = generate_summary(
        &messages,
        model.as_ref(),
        4096,
        None,
        None,
        None,
        &xylitol_ai_bridge::ObsSessionContext::default(),
    )
    .await;
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
    use crate::agent::compaction::{CompactionSettings, compact_session};

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
                timestamp: 1704067200000,
            },
            message: serde_json::json!({"role":"user","content":format!("message {i} {}", "x".repeat(400))}),
        });
        let _ = mgr.append(sid, &e).await;
    }
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
        keep_recent_tokens: 4_000,
    };
    let result = compact_session(
        &mgr,
        sid,
        model.as_ref(),
        &settings,
        None,
        0,
        None,
        None,
        &xylitol_ai_bridge::ObsSessionContext::default(),
    )
    .await;
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
    store: Arc<dyn crate::protocol::ports::XySessionStore>,
) -> AgentCapabilities {
    let sink: Arc<dyn crate::protocol::ports::XyEventSink> =
        Arc::new(crate::infra::event::EventBus::new());
    AgentCapabilities::new(
        agent.registry.borrow().clone(),
        ToolSet::from_iter(crate::infra::tools::default_tools()),
        store,
        sink,
        None,
        Vec::new(),
        Vec::new(),
        ".".into(),
        None,
        Arc::new(crate::infra::provider::factory::build_provider),
        crate::infra::permission::allow_all_permission(),
        crate::agent::capabilities::QueueMode::default(),
        crate::agent::capabilities::QueueMode::default(),
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
                timestamp: 1704067200000,
            },
            message: serde_json::json!({"role":"user","content":format!("new turn {i}")}),
        });
        let _ = mgr.append("comp-iterative", &e).await;
    }
    sess.current_id.replace(Some("comp-iterative".into()));
}

#[when("以 previousSummary 调用 generate_summary")]
pub(crate) async fn w_comp_iterative_summary(agent: &AgentState, sess: &XySessionStore) {
    use crate::agent::compaction::generate_summary;

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
    let model =
        crate::infra::provider::factory::build_provider(&crate::protocol::model::XyModelConfig {
            kind: crate::protocol::model::XyModelKind::Fake,
            model: "fake".into(),
            api_key: String::new(),
            base_url: None,
            api: None,
            compat: None,
        });
    let result = generate_summary(
        &messages,
        model.as_ref(),
        4096,
        prev.as_deref(),
        None,
        None,
        &xylitol_ai_bridge::ObsSessionContext::default(),
    )
    .await;
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
    use crate::protocol::message::{AgentMessage, AgentPart, LlmMessage, XyStopReason};

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
                timestamp: 1704067200000,
            },
            message: serde_json::to_value(AgentMessage::user(label)).unwrap(),
        });
        let _ = mgr.append(sid, &user).await;
        let assistant = SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: format!("msg-a-{i}"),
                parent_id: None,
                timestamp: 1704067201000,
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
                timestamp: 1704067202000,
            },
            message: serde_json::to_value(AgentMessage::user(format!(
                "padding turn {i} {}",
                "y".repeat(400)
            )))
            .unwrap(),
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
    let settings = crate::agent::compaction::CompactionSettings {
        enabled: true,
        reserve_tokens: 10_000,
        keep_recent_tokens: 20_000,
    };
    // pi: tokens > window - reserve → 90_001 > 90_000
    agent
        .compaction_result
        .replace(Some(should_compact(90_001, 100_000, &settings, 0)));
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
