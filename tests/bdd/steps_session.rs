use crate::fixtures::*;
use crate::helpers::{result_ok_str, strip_quotes};
use crate::prelude::*;
use rstest_bdd_macros::{given, then, when};

fn bdd_provider_for(model: &str) -> &'static str {
    let m = model.to_ascii_lowercase();
    if m.contains("claude") || m.contains("sonnet") || m.contains("anthropic") {
        "anthropic"
    } else if m.starts_with("gpt") || m.contains("openai") {
        "openai"
    } else {
        "unknown"
    }
}

fn message_entry(id: &str, parent_id: Option<&str>, role: &str, content: &str) -> SessionEntry {
    SessionEntry::Message(MessageEntry {
        base: EntryBase {
            entry_type: "message".into(),
            id: id.into(),
            parent_id: parent_id.map(str::to_string),
            timestamp: "2024-01-01T00:00:00Z".into(),
        },
        message: serde_json::json!({"role": role, "content": content}),
    })
}

async fn seed_linked_messages(mgr: &SessionManager, sid: &str, count: u32) {
    for i in 0..count {
        let id = format!("msg-{i}");
        let parent = if i == 0 {
            None
        } else {
            Some(format!("msg-{}", i - 1))
        };
        // Mix in an assistant on the last slot so Persisted fork can flush to disk.
        let (role, content) = if i + 1 == count && count > 1 {
            ("assistant", format!("reply {i}"))
        } else {
            ("user", format!("message {i}"))
        };
        mgr.append_with_id(sid, &message_entry(&id, parent.as_deref(), role, &content))
            .await
            .unwrap();
    }
}

async fn append_branch_summary(
    mgr: &SessionManager,
    parent_id: &str,
    child_id: &str,
    at_entry_id: &str,
    label: &str,
) {
    let parent_entries = mgr.load(parent_id).await.unwrap();
    let child_path = mgr
        .get_branch(child_id, mgr.get_leaf_id(child_id).as_deref())
        .await
        .unwrap_or_default();
    let child_ids: std::collections::HashSet<&str> =
        child_path.iter().filter_map(|e| e.entry_id()).collect();
    let skipped: Vec<SessionEntry> = parent_entries
        .iter()
        .filter(|e| matches!(e, SessionEntry::Message(_)))
        .filter(|e| e.entry_id().is_some_and(|id| !child_ids.contains(id)))
        .cloned()
        .collect();
    let mut summary = mgr.generate_branch_summary(&skipped);
    if summary.is_empty() {
        summary = format!("summary for {label}");
    } else {
        summary = format!("{label}: {summary}");
    }
    let leaf = mgr.get_leaf_id(child_id);
    mgr.append_with_id(
        child_id,
        &SessionEntry::BranchSummary(BranchSummaryEntry {
            base: EntryBase {
                entry_type: "branchSummary".into(),
                id: format!("bs-{child_id}"),
                parent_id: leaf,
                timestamp: "2024-01-01T00:00:10Z".into(),
            },
            from_id: at_entry_id.into(),
            summary,
            details: None,
            from_hook: None,
        }),
    )
    .await
    .unwrap();
}

async fn fork_with_branch_summary(
    mgr: &SessionManager,
    parent_id: &str,
    child_id: &str,
    at_entry_id: &str,
    label: &str,
) {
    mgr.fork(parent_id, child_id, at_entry_id, ForkPosition::At)
        .await
        .unwrap_or_else(|e| panic!("fork {parent_id}->{child_id} at {at_entry_id}: {e}"));
    append_branch_summary(mgr, parent_id, child_id, at_entry_id, label).await;
}

fn count_messages(entries: &[SessionEntry]) -> usize {
    entries
        .iter()
        .filter(|e| matches!(e, SessionEntry::Message(_)))
        .count()
}

#[given("存在会话 {id:string}")]
async fn _g_session_exists(sess: &XySessionStore, id: String) {
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let _ = mgr.create(&id, Some("."), None).await;
    sess.current_id.replace(Some(id));
}

#[given("存在会话 {id:string} 包含 {count:u32} 条记录")]
async fn _g_session_with_n(sess: &XySessionStore, id: String, count: u32) {
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let _ = mgr.create(&id, Some("."), None).await;
    seed_linked_messages(&mgr, &id, count).await;
    sess.current_id.replace(Some(id));
}

#[when("创建一个新会话 {id:string}")]
async fn _w_session_create(sess: &XySessionStore, id: String) {
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    mgr.create(&id, Some("."), None).await.unwrap();
    sess.current_id.replace(Some(id));
}

#[when("向会话追加一条消息 {msg:string}")]
async fn _w_session_append(sess: &XySessionStore, msg: String) {
    sess.ensure_mgr();
    let sid = sess.current_id.borrow().clone().unwrap();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let e = SessionEntry::Message(MessageEntry {
        base: EntryBase {
            entry_type: "message".into(),
            id: "msg-1".into(),
            parent_id: None,
            timestamp: "2024-01-01T00:00:00Z".into(),
        },
        message: serde_json::json!({"role":"user","content":msg}),
    });
    mgr.append(&sid, &e).await.unwrap();
}

#[when("加载会话 {id:string}")]
async fn _w_session_load(sess: &XySessionStore, id: String) {
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let entries = mgr.load(&id).await.unwrap();
    sess.entries.replace(entries);
}

#[then("会话包含 {count:u32} 条记录")]
fn _t_session_has_n(sess: &XySessionStore, count: u32) {
    let n = sess
        .entries
        .borrow()
        .iter()
        .filter(|e| e.entry_type() != "session")
        .count();
    assert_eq!(n, count as usize);
}

#[then("记录类型为 {typ:string}")]
fn _t_session_entry_type(sess: &XySessionStore, typ: String) {
    assert!(
        sess.entries
            .borrow()
            .iter()
            .skip(1)
            .any(|e| e.entry_type() == typ)
    );
}

#[when("列出所有会话")]
async fn _w_session_list(ws: &Workspace, sess: &XySessionStore) {
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let ids = mgr.list().await.unwrap_or_default();
    ws.last_result.replace(Some(Ok(ids.join("\n"))));
}

#[when("在记录 {n:u32} 处分叉创建会话 {id:string}")]
async fn _w_session_fork(sess: &XySessionStore, n: u32, id: String) {
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let parent = sess
        .current_id
        .borrow()
        .clone()
        .unwrap_or_else(|| "parent".into());
    assert!(n >= 1, "fork record index is 1-based");
    let at = format!("msg-{}", n - 1);
    fork_with_branch_summary(&mgr, &parent, &id, &at, &id).await;
    sess.current_id.replace(Some(id.clone()));
    sess.entries.replace(mgr.load(&id).await.unwrap());
}

#[given("存在会话树: {tree}")]
async fn _g_session_tree(sess: &XySessionStore, tree: String) {
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let chain: Vec<String> = tree
        .split('→')
        .map(|s| s.trim().trim_matches('"').to_string())
        .filter(|s| !s.is_empty())
        .collect();
    assert!(
        chain.len() >= 2,
        "expected session tree chain with ≥2 ids, got {tree:?}"
    );
    let root = &chain[0];
    mgr.create(root, Some("."), None).await.unwrap();
    seed_linked_messages(&mgr, root, 6).await;
    let mut parent = root.clone();
    for (i, child) in chain.iter().skip(1).enumerate() {
        // Fork deeper each hop so each child keeps a distinct path + summary label.
        let at = format!("msg-{}", 4usize.saturating_sub(i));
        fork_with_branch_summary(&mgr, &parent, child, &at, child).await;
        parent = child.clone();
    }
    let leaf = chain.last().unwrap().clone();
    sess.current_id.replace(Some(leaf.clone()));
    sess.entries.replace(mgr.load(&leaf).await.unwrap());
}

#[when("导航到 {target}")]
async fn _w_session_nav_to(sess: &XySessionStore, target: String) {
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let target = strip_quotes(&target);
    mgr.set_active_session(&target);
    if let Some(leaf) = mgr.get_leaf_id(&target) {
        mgr.navigate_tree(&target, Some(&leaf));
    }
    let entries = mgr.load(&target).await.unwrap();
    sess.current_id.replace(Some(target));
    sess.entries.replace(entries);
}

#[when("将会话模型从 {from} 切换为 {to}")]
async fn _w_session_model_switch(sess: &XySessionStore, from: String, to: String) {
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let sid = sess
        .current_id
        .borrow()
        .clone()
        .expect("current session for model switch");
    let _ = from;
    let to = strip_quotes(&to);
    mgr.append_model_change(&sid, bdd_provider_for(&to), &to)
        .await
        .unwrap();
    sess.entries.replace(mgr.load(&sid).await.unwrap());
}

#[when("切换思考级别为 {level}")]
async fn _w_session_thinking_switch(sess: &XySessionStore, level: String) {
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let sid = sess
        .current_id
        .borrow()
        .clone()
        .expect("current session for thinking switch");
    let level = strip_quotes(&level);
    mgr.append_thinking_level_change(&sid, &level)
        .await
        .unwrap();
    sess.entries.replace(mgr.load(&sid).await.unwrap());
}

#[when("向会话追加 {n:u32} 条不同类型的记录")]
async fn _w_session_append_n(sess: &XySessionStore, n: u32) {
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let sid = sess
        .current_id
        .borrow()
        .clone()
        .expect("current session for append_n");
    // Ensure at least message / modelChange / thinkingLevelChange when n>=3.
    let kinds = n.max(1);
    for i in 0..kinds {
        match i % 3 {
            0 => {
                let id = format!("extra-msg-{i}");
                mgr.append_with_id(
                    &sid,
                    &message_entry(
                        &id,
                        mgr.get_leaf_id(&sid).as_deref(),
                        "user",
                        &format!("x{i}"),
                    ),
                )
                .await
                .unwrap();
            }
            1 => {
                mgr.append_model_change(&sid, "openai", "gpt-4o")
                    .await
                    .unwrap();
            }
            _ => {
                mgr.append_thinking_level_change(&sid, "high")
                    .await
                    .unwrap();
            }
        }
    }
    // Flush assistant so JSONL exists on disk for format asserts.
    let leaf = mgr.get_leaf_id(&sid);
    mgr.append_with_id(
        &sid,
        &message_entry("flush-asst", leaf.as_deref(), "assistant", "ok"),
    )
    .await
    .unwrap();
    sess.entries.replace(mgr.load(&sid).await.unwrap());
}

#[then("会话 {id:string} 包含 {count:u32} 条记录")]
async fn _t_session_id_has_n(sess: &XySessionStore, id: String, count: u32) {
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let entries = mgr.load(&id).await.unwrap();
    assert_eq!(
        count_messages(&entries),
        count as usize,
        "session {id} message count mismatch; entries={entries:?}"
    );
}

#[then("会话 {id:string} 包含一个 branch_summary 记录")]
async fn _t_session_has_branch_summary(sess: &XySessionStore, id: String) {
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let entries = mgr.load(&id).await.unwrap();
    assert!(
        entries
            .iter()
            .any(|e| matches!(e, SessionEntry::BranchSummary(_))),
        "expected branchSummary in {id}, got {entries:?}"
    );
}

#[then("会话包含 model_change 记录")]
fn _t_session_has_model_change(sess: &XySessionStore) {
    assert!(
        sess.entries
            .borrow()
            .iter()
            .any(|e| matches!(e, SessionEntry::ModelChange(_))),
        "expected modelChange entry, got {:?}",
        sess.entries.borrow()
    );
}

#[then("model_change 记录显示 provider 为 {provider}")]
fn _t_session_model_change_provider(sess: &XySessionStore, provider: String) {
    let provider = strip_quotes(&provider);
    let found = sess.entries.borrow().iter().find_map(|e| match e {
        SessionEntry::ModelChange(mc) => Some(mc.provider.clone()),
        _ => None,
    });
    assert_eq!(
        found.as_deref(),
        Some(provider.as_str()),
        "modelChange provider mismatch; entries={:?}",
        sess.entries.borrow()
    );
}

#[then("会话包含 thinking_level_change 记录")]
fn _t_session_has_thinking_change(sess: &XySessionStore) {
    assert!(
        sess.entries
            .borrow()
            .iter()
            .any(|e| matches!(e, SessionEntry::ThinkingLevelChange(_))),
        "expected thinkingLevelChange entry, got {:?}",
        sess.entries.borrow()
    );
}

#[then("JSONL 文件每行是一个完整的 JSON 对象")]
fn _t_session_jsonl_lines(sess: &XySessionStore) {
    let mgr = sess.mgr.borrow().as_ref().expect("session manager").clone();
    let sid = sess
        .current_id
        .borrow()
        .clone()
        .expect("current session for jsonl assert");
    let path = mgr
        .get_session_file(&sid)
        .expect("persisted session jsonl path");
    let content = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!("read jsonl {}: {e}", path.display());
    });
    assert!(!content.trim().is_empty(), "jsonl must be non-empty");
    for (i, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        serde_json::from_str::<serde_json::Value>(line)
            .unwrap_or_else(|e| panic!("line {i} not JSON: {e}; line={line}"));
    }
}

#[then("第一行包含 version 字段")]
fn _t_session_jsonl_version(sess: &XySessionStore) {
    let mgr = sess.mgr.borrow().as_ref().expect("session manager").clone();
    let sid = sess
        .current_id
        .borrow()
        .clone()
        .expect("current session for version assert");
    let path = mgr
        .get_session_file(&sid)
        .expect("persisted session jsonl path");
    let content = std::fs::read_to_string(&path).unwrap();
    let first = content
        .lines()
        .find(|l| !l.trim().is_empty())
        .expect("jsonl first line");
    let v: serde_json::Value = serde_json::from_str(first).unwrap();
    assert!(
        v.get("version").is_some(),
        "first jsonl line must include version, got {first}"
    );
}

#[then("上下文包含 branch-a 和 branch-b 的摘要")]
async fn _t_session_context_branches(sess: &XySessionStore) {
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let mut sid = sess
        .current_id
        .borrow()
        .clone()
        .expect("current session for tree context");
    let mut summaries = Vec::new();
    // Walk leaf → parents via header.parent_session and collect BranchSummary texts.
    loop {
        let entries = mgr.load(&sid).await.unwrap();
        for e in &entries {
            if let SessionEntry::BranchSummary(b) = e {
                summaries.push(b.summary.clone());
            }
        }
        let parent = entries.iter().find_map(|e| match e {
            SessionEntry::Header(h) => h.parent_session.clone(),
            _ => None,
        });
        match parent {
            Some(p) => sid = p,
            None => break,
        }
    }
    assert!(
        summaries.iter().any(|s| s.contains("branch-a")),
        "expected branch-a summary along parent chain, got {summaries:?}"
    );
    assert!(
        summaries.iter().any(|s| s.contains("branch-b")),
        "expected branch-b summary along parent chain, got {summaries:?}"
    );
}

// ── Label and session_info steps ──

#[given("向会话追加一条消息 {msg:string}")]
async fn _given_session_append_msg(sess: &XySessionStore, msg: String) {
    _w_session_append(sess, msg).await;
}

#[when("为最后一条记录设置标签 {label:string}")]
async fn _w_session_set_label(sess: &XySessionStore, label: String) {
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let sid = sess.current_id.borrow().as_ref().unwrap().clone();
    let entries = mgr.load(&sid).await.unwrap();
    let last_id = entries
        .iter()
        .rev()
        .find(|e| {
            e.entry_type() != "label"
                && e.entry_type() != "sessionInfo"
                && e.entry_type() != "session"
        })
        .and_then(|e| e.entry_id().map(String::from))
        .expect("no entries to label");
    mgr.append_label_change(&sid, &last_id, Some(&label))
        .await
        .unwrap();
}

#[when("清除该记录的标签")]
async fn _w_session_clear_label(sess: &XySessionStore) {
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let sid = sess.current_id.borrow().as_ref().unwrap().clone();
    let entries = mgr.load(&sid).await.unwrap();
    let last_id = entries
        .iter()
        .rev()
        .find(|e| {
            e.entry_type() != "label"
                && e.entry_type() != "sessionInfo"
                && e.entry_type() != "session"
        })
        .and_then(|e| e.entry_id().map(String::from))
        .expect("no entries to clear label");
    mgr.append_label_change(&sid, &last_id, None).await.unwrap();
}

#[then("该记录的标签为 {expected:string}")]
async fn _t_session_label_is(sess: &XySessionStore, expected: String) {
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let sid = sess.current_id.borrow().as_ref().unwrap().clone();
    let entries = mgr.load(&sid).await.unwrap();
    let last_id = entries
        .iter()
        .rev()
        .find(|e| {
            e.entry_type() != "label"
                && e.entry_type() != "sessionInfo"
                && e.entry_type() != "session"
        })
        .and_then(|e| e.entry_id())
        .expect("no entries");
    let label = mgr.get_label(&sid, last_id).await.unwrap();
    assert_eq!(label, Some(expected), "label mismatch");
}

#[then("该记录没有标签")]
async fn _t_session_no_label(sess: &XySessionStore) {
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let sid = sess.current_id.borrow().as_ref().unwrap().clone();
    let entries = mgr.load(&sid).await.unwrap();
    let last_id = entries
        .iter()
        .rev()
        .find(|e| {
            e.entry_type() != "label"
                && e.entry_type() != "sessionInfo"
                && e.entry_type() != "session"
        })
        .and_then(|e| e.entry_id())
        .expect("no entries");
    let label = mgr.get_label(&sid, last_id).await.unwrap();
    assert!(label.is_none(), "expected no label, got {:?}", label);
}

#[then("恰好有 {n:u32} 条匹配")]
fn _t_grep_exact_matches(ws: &Workspace, n: u32) {
    let r = result_ok_str(&ws.last_result);
    let count = r
        .lines()
        .filter(|l| {
            let parts: Vec<_> = l.splitn(3, ':').collect();
            parts.len() >= 3 && parts[1].parse::<u32>().is_ok()
        })
        .count();
    assert_eq!(
        count, n as usize,
        "expected exactly {n} matches, got {count} in:\n{r}"
    );
}
