use crate::bdd::fixtures::*;
use crate::bdd::helpers::{result_ok_str, strip_quotes};
use crate::bdd::prelude::*;
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
            timestamp: 1704067200000,
        },
        // dm1：content 必须是带 type 判别的部件数组；裸字符串按 c646 不可读。
        message: serde_json::json!({"role": role, "content": [{"type": "text", "text": content}]}),
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
                timestamp: 1704067210000,
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
            timestamp: 1704067200000,
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

// ---- agent-session-store 转写批（s9 / s10 / s11 / s12 / s21 / s22）----

fn legacy_session_file(dir: &std::path::Path, id: &str) -> std::path::PathBuf {
    dir.join(format!("{id}.jsonl"))
}

fn sess_dir(sess: &XySessionStore) -> std::path::PathBuf {
    sess.sessions_dir
        .borrow()
        .clone()
        .expect("sessions dir captured by ensure_mgr")
}

fn active_session_file(sess: &XySessionStore, id: &str) -> std::path::PathBuf {
    sess.mgr
        .borrow()
        .as_ref()
        .expect("session manager captured by ensure_mgr")
        .get_session_file(id)
        .expect("persisted session active segment path")
}

async fn append_typed_message(sess: &XySessionStore, role: &str, msg: &str) {
    sess.ensure_mgr();
    let id = sess.current_id.borrow().clone().expect("current session");
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let n = mgr.load(&id).await.unwrap_or_default().len();
    let entry = SessionEntry::Message(MessageEntry {
        base: EntryBase {
            entry_type: "message".into(),
            id: format!("{role}-{n}"),
            parent_id: None,
            timestamp: 1704067200000,
        },
        // dm1 合法形态（同 message_entry）。
        message: serde_json::json!({
            "role": role,
            "content": [{"type": "text", "text": msg}]
        }),
    });
    mgr.append_with_id(&id, &entry).await.unwrap();
}

#[when("向会话追加用户消息 {msg:string}")]
async fn when_append_user_msg(sess: &XySessionStore, msg: String) {
    append_typed_message(sess, "user", &msg).await;
}

#[when("向会话追加助手消息 {msg:string}")]
async fn when_append_assistant_msg(sess: &XySessionStore, msg: String) {
    append_typed_message(sess, "assistant", &msg).await;
}

#[then("会话 {id:string} 的磁盘 JSONL 尚不存在")]
async fn then_disk_absent(sess: &XySessionStore, id: String) {
    let p = active_session_file(sess, &id);
    let session_dir = sess_dir(sess).join(&id);
    let legacy = legacy_session_file(&sess_dir(sess), &id);
    assert!(
        !p.exists() && !session_dir.exists() && !legacy.exists(),
        "s12: header/entries must stay deferred before first append, found active={}, dir={}, legacy={}",
        p.display(),
        session_dir.display(),
        legacy.display()
    );
}

#[then("会话 {id:string} 的磁盘 JSONL 已存在且包含 {text:string}")]
async fn then_disk_contains(sess: &XySessionStore, id: String, text: String) {
    let p = active_session_file(sess, &id);
    assert!(p.exists(), "s12: disk jsonl must exist, {}", p.display());
    let body = std::fs::read_to_string(&p).unwrap();
    assert!(body.contains(&text), "{text} missing in:\n{body}");
    let first = body.lines().next().expect("header line");
    assert!(
        first.contains("\"session\""),
        "first line must be the header entry: {first}"
    );
}

fn serialized_entries(entries: &[SessionEntry]) -> Vec<String> {
    entries
        .iter()
        .map(|e| serde_json::to_string(e).expect("serialize entry"))
        .collect()
}

#[then("会话 {id:string} 包含文本 {text:string}")]
async fn then_session_has_text(sess: &XySessionStore, id: String, text: String) {
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let all = serialized_entries(&mgr.load(&id).await.unwrap());
    assert!(
        all.iter().any(|s| s.contains(&text)),
        "{text} missing in {} entries:\n{}",
        all.len(),
        all.join("\n")
    );
}

#[then("会话 {id:string} 不含文本 {text:string}")]
async fn then_session_lacks_text(sess: &XySessionStore, id: String, text: String) {
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let all = serialized_entries(&mgr.load(&id).await.unwrap());
    assert!(
        all.iter().all(|s| !s.contains(&text)),
        "s9: forked child must not contain post-cutoff text {text}:\n{}",
        all.join("\n")
    );
}

#[then("会话 {id:string} 的 branch_summary 摘要包含 {text:string}")]
async fn then_summary_mentions(sess: &XySessionStore, id: String, text: String) {
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let entries = mgr.load(&id).await.unwrap();
    let summary = entries
        .iter()
        .find_map(|e| match e {
            SessionEntry::BranchSummary(b) => Some(b.summary.clone()),
            _ => None,
        })
        .expect("branch_summary entry");
    assert!(
        summary.contains(&text),
        "s10: summary must mention {text:?}, got {summary:?}"
    );
}

#[when("目录中植入损坏的会话文件 broken.jsonl")]
fn when_plant_corrupt_file(sess: &XySessionStore) {
    let p = legacy_session_file(&sess_dir(sess), "broken");
    std::fs::write(&p, "{ not json\n").expect("plant corrupt file");
}

#[when("目录中植入旧格式会话文件 {id:string}")]
fn when_plant_legacy_session_file(sess: &XySessionStore, id: String) {
    // A v6-style single-file session with no v7 manifest (zero-compat boundary).
    sess.ensure_mgr();
    let dir = sess_dir(sess);
    std::fs::create_dir_all(&dir).ok();
    let p = legacy_session_file(&dir, &id);
    let header = serde_json::json!({
        "type": "session",
        "version": 6,
        "id": id,
        "timestamp": 1,
        "cwd": "."
    });
    std::fs::write(&p, format!("{header}\n")).expect("plant legacy session file");
}

#[then("会话 {id:string} 的 JSONL 时间戳均为 u64 毫秒")]
async fn then_timestamps_u64_ms(sess: &XySessionStore, id: String) {
    let p = active_session_file(sess, &id);
    let body = std::fs::read_to_string(&p).unwrap();
    for (i, line) in body.lines().enumerate() {
        let v: serde_json::Value =
            serde_json::from_str(line).unwrap_or_else(|e| panic!("line {i}: {e}"));
        let ts = v
            .get("timestamp")
            .unwrap_or_else(|| panic!("s22: line {i} must carry shell timestamp:\n{line}"));
        assert!(
            ts.is_u64() && ts.as_u64().unwrap() >= 1_000_000_000_000,
            "s22: line {i} timestamp must be u64 unix-ms, got {ts}"
        );
    }
}

// ---- ex1–ex4：导出/导入四连（SessionExporter 纯协作者直驱）----

fn tmp_root(sess: &XySessionStore) -> std::path::PathBuf {
    sess_dir(sess)
        .parent()
        .map(std::path::Path::to_path_buf)
        .expect("leaked tempdir root")
}

fn exporter() -> xylitol::SessionExporter {
    use xylitol::infra::export::StdExportIo;
    xylitol::SessionExporter::new(Some(std::sync::Arc::new(StdExportIo::new())))
}

async fn append_bash_execution(sess: &XySessionStore, cmd: &str, out: &str) {
    sess.ensure_mgr();
    let id = sess.current_id.borrow().clone().expect("current session");
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let n = mgr.load(&id).await.unwrap_or_default().len();
    let entry = SessionEntry::Message(MessageEntry {
        base: EntryBase {
            entry_type: "message".into(),
            id: format!("bash-{n}"),
            parent_id: None,
            timestamp: 1704067200000,
        },
        message: serde_json::json!({
            "role": "bashExecution",
            "command": cmd,
            "output": out
        }),
    });
    mgr.append_with_id(&id, &entry).await.unwrap();
}

#[when("导出会话 {id:string} 为 JSONL 文件 {file:string}")]
async fn when_export_jsonl(sess: &XySessionStore, id: String, file: String) {
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let path = tmp_root(sess).join(&file);
    exporter()
        .export_to_jsonl(&mgr, &id, &path)
        .await
        .unwrap_or_else(|e| panic!("ex2 export jsonl: {e}"));
}

#[then("导出文件 {file:string} 包含 {text:string}")]
fn then_export_file_contains(sess: &XySessionStore, file: String, text: String) {
    let body = std::fs::read_to_string(tmp_root(sess).join(&file))
        .unwrap_or_else(|e| panic!("ex2: {file}: {e}"));
    assert!(body.contains(&text), "ex2: {text:?} missing in:\n{body}");
}

#[when("把 JSONL 文件 {file:string} 导入全新会话存储为 {id:string}")]
async fn when_import_jsonl_fresh_store(sess: &XySessionStore, file: String, id: String) {
    let root = tmp_root(sess);
    let fresh = SessionManager::new(root.join("imported-sessions"));
    let returned = exporter()
        .import_from_jsonl(&fresh, &root.join(&file))
        .await
        .unwrap_or_else(|e| panic!("ex3 import jsonl: {e}"));
    assert_eq!(returned, id, "ex3: header-reused session id must match");
    sess.second_mgr.replace(Some(fresh));
}

#[then("导入存储中会话 {id:string} 包含文本 {text:string}")]
async fn then_imported_has_text(sess: &XySessionStore, id: String, text: String) {
    let mgr = sess
        .second_mgr
        .borrow()
        .as_ref()
        .expect("imported store")
        .clone();
    let all = serialized_entries(&mgr.load(&id).await.unwrap());
    assert!(
        all.iter().any(|s| s.contains(&text)),
        "ex3: imported entries must contain {text:?}:\n{}",
        all.join("\n")
    );
}

#[when("导出会话 {id:string} 为 HTML 文件 {file:string}")]
async fn when_export_html(sess: &XySessionStore, id: String, file: String) {
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let path = tmp_root(sess).join(&file);
    exporter()
        .export_to_html(&mgr, &id, &path)
        .await
        .unwrap_or_else(|e| panic!("ex1 export html: {e}"));
}

#[then("HTML 文件 {file:string} 含可读块文本 {text:string}")]
fn then_html_block_text(sess: &XySessionStore, file: String, text: String) {
    let html = std::fs::read_to_string(tmp_root(sess).join(&file))
        .unwrap_or_else(|e| panic!("ex4: {file}: {e}"));
    assert!(
        html.starts_with("<!doctype html>"),
        "standalone document required"
    );
    assert!(
        html.contains(&text),
        "ex4: readable block must contain {text:?}"
    );
}

#[when("向会话追加 bash 执行记录（命令 {cmd:string} 输出 {out:string}）")]
async fn when_append_bash_record(sess: &XySessionStore, cmd: String, out: String) {
    append_bash_execution(sess, &cmd, &out).await;
}

// ---- sc1 / sc2 / s16：CWD 校验三面（load_validated 同一 seam）----
// sc3/sc4 属呈现与 CLI/RPC 接线条款，生产暂无调用方，维持 @human。

#[when("创建存储于目录 {cwd:string} 的新会话 {id:string}")]
async fn when_create_with_cwd(sess: &XySessionStore, cwd: String, id: String) {
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let _ = mgr.create(&id, Some(&cwd), None).await;
    sess.current_id.replace(Some(id.clone()));
}

#[then("校验加载 {id:string} 回退 {fb:string} 成功且非空")]
async fn then_cwd_validate_ok(sess: &XySessionStore, id: String, fb: String) {
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let entries = mgr
        .load_validated(&id, &fb)
        .await
        .unwrap_or_else(|e| panic!("sc1: fallback must rescue, got {e}"));
    assert!(
        !entries.is_empty(),
        "sc1: validated load must return entries"
    );
}

#[then("校验加载 {id:string} 回退 {fb:string} 失败并提及 {a:string} 与 {b:string}")]
async fn then_cwd_validate_err_both(
    sess: &XySessionStore,
    id: String,
    fb: String,
    a: String,
    b: String,
) {
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let err = mgr
        .load_validated(&id, &fb)
        .await
        .expect_err("sc2: both missing must fail");
    let msg = err.to_string();
    assert!(msg.contains(&a), "sc2: stored cwd in message: {msg}");
    assert!(msg.contains(&b), "sc2: fallback cwd in message: {msg}");
}

// ---- s19：toolResult 以 toolCallId 键持久化（对齐 pi）----

#[when("向会话追加关联 {call_id:string} 的工具结果消息 {text:string}")]
async fn when_append_tool_result(sess: &XySessionStore, call_id: String, text: String) {
    use xylitol::protocol::message::{AgentMessage, AgentPart};
    sess.ensure_mgr();
    let id = sess.current_id.borrow().clone().expect("current session");
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let n = mgr.load(&id).await.unwrap_or_default().len();
    let am = AgentMessage::tool_result(call_id, "read", vec![AgentPart::text(text)], false);
    let entry = SessionEntry::Message(MessageEntry {
        base: EntryBase {
            entry_type: "message".into(),
            id: format!("tool-{n}"),
            parent_id: None,
            timestamp: 1704067200000,
        },
        message: serde_json::to_value(&am).expect("serialize AgentMessage"),
    });
    mgr.append_with_id(&id, &entry).await.unwrap();
}

#[then("会话 {id:string} 的磁盘行含 toolCallId 且不含 toolUseId")]
async fn then_tool_call_id_key(sess: &XySessionStore, id: String) {
    let p = active_session_file(sess, &id);
    let body = std::fs::read_to_string(&p).unwrap();
    let hit = body
        .lines()
        .find(|l| l.contains("\"toolResult\""))
        .expect("s19: persisted toolResult line");
    assert!(hit.contains("\"toolCallId\""), "s19: {hit}");
    assert!(!hit.contains("toolUseId"), "s19: legacy key leaked: {hit}");
}

// ---- s6：分支摘要生成器边界（空切点 ⇒ 空摘要）----

#[when("调用分支摘要生成于空切点集合")]
async fn when_summary_empty_set(sess: &XySessionStore) {
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let summary = mgr.generate_branch_summary(&[]);
    sess.last_result
        .replace(Some(Ok(format!("summary-len={}", summary.chars().count()))));
}

#[then("分支摘要为空字符串")]
async fn then_summary_empty(sess: &XySessionStore) {
    let text = {
        let b = sess.last_result.borrow();
        let r = b.as_ref().expect("summary captured");
        match r {
            Ok(t) => t.clone(),
            Err(e) => panic!("s6: {e}"),
        }
    };
    assert_eq!(
        text.trim(),
        "summary-len=0",
        "s6: empty input => empty output"
    );
}

// ---- c2826 specs-compact：裸规则转场景补充步骤 ----

/// 植入一个 v7 会话（2 条合法 user 消息）并向 active 段直接追加一行自定义内容。
async fn plant_v7_with_extra_line(sess: &XySessionStore, id: &str, extra_line: &str) {
    sess.ensure_mgr();
    sess.current_id.replace(Some(id.to_string()));
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    mgr.create(id, Some("."), None).await.unwrap();
    for (n, text) in ["第一条", "第二条"].iter().enumerate() {
        let entry = SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: format!("plant-{n}"),
                parent_id: None,
                timestamp: 1704067200000,
            },
            message: serde_json::json!({
                "role": "user",
                "content": [{"type": "text", "text": text}]
            }),
        });
        mgr.append_with_id(id, &entry).await.unwrap();
    }
    let path = active_session_file(sess, id);
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap();
    writeln!(f, "{extra_line}").unwrap();
}

#[given("目录中植入含坏 JSON 行的 v7 会话 {id:string}")]
async fn given_plant_bad_json_line(sess: &XySessionStore, id: String) {
    let id = strip_quotes(&id);
    plant_v7_with_extra_line(sess, &id, "not-json{{broken").await;
}

#[given("目录中植入含旧 untagged content 行的 v7 会话 {id:string}")]
async fn given_plant_legacy_untagged(sess: &XySessionStore, id: String) {
    let id = strip_quotes(&id);
    // 旧 c646 前形态：content 为裸字符串而非 dm1 tagged 数组。
    let line = r#"{"type":"message","id":"legacy-1","timestamp":1,"message":{"role":"user","content":"plain old string"}}"#;
    plant_v7_with_extra_line(sess, &id, line).await;
}

#[then("坏行被跳过且其余条目正常加载")]
async fn then_bad_line_skipped(sess: &XySessionStore) {
    let entries = sess.entries.borrow();
    let texts: Vec<String> = entries
        .iter()
        .filter_map(|e| match e {
            SessionEntry::Message(m) => m
                .message
                .get("content")
                .and_then(|c| c.as_array())
                .and_then(|a| a.first())
                .and_then(|p| p.get("text"))
                .and_then(|t| t.as_str())
                .map(str::to_owned),
            _ => None,
        })
        .collect();
    assert_eq!(
        texts,
        vec!["第一条".to_string(), "第二条".to_string()],
        "c2826: 坏行必须被跳过且好行全部加载"
    );
}

#[when("目录中植入未引用的临时段文件")]
async fn when_plant_orphan_segment(sess: &XySessionStore) {
    let id = sess.current_id.borrow().clone().expect("current session");
    let dir = sess_dir(sess).join(&id).join("segments");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("00000000000000000099-sealed.jsonl"),
        "{\"type\":\"message\",\"id\":\"orphan\",\"message\":{\"role\":\"user\",\"content\":[]}}\n",
    )
    .unwrap();
}

#[when("破坏该会话的 sealed sidecar 索引文件")]
async fn when_corrupt_sidecar(sess: &XySessionStore) {
    let id = sess.current_id.borrow().clone().expect("current session");
    let manifest_path = sess_dir(sess).join(&id).join("manifest.json");
    let raw = std::fs::read(&manifest_path).expect("manifest exists after seal");
    let manifest: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    let index_path = manifest["sealedSegments"]
        .as_array()
        .expect("sealed segments recorded")
        .last()
        .expect("compaction sealed a cold segment")
        .get("indexPath")
        .and_then(|v| v.as_str())
        .expect("sidecar indexPath recorded")
        .to_string();
    let sidecar = sess_dir(sess).join(&id).join(&index_path);
    std::fs::write(&sidecar, "not-json{corrupt").unwrap();
}

fn first_disk_line(sess: &XySessionStore, id: &str) -> serde_json::Value {
    let path = active_session_file(sess, id);
    let raw = std::fs::read_to_string(&path).expect("active segment readable");
    let line = raw.lines().next().expect("header line present");
    serde_json::from_str(line).expect("header line is JSON")
}

#[then("会话 {id:string} 头含父会话与切点条目 id")]
async fn then_header_carries_cut(sess: &XySessionStore, id: String) {
    let id = strip_quotes(&id);
    let header = first_disk_line(sess, &id);
    let parent = header.get("parentSession").and_then(|v| v.as_str());
    let cut = header.get("forkAtEntryId").and_then(|v| v.as_str());
    assert!(
        parent.is_some_and(|p| !p.is_empty()),
        "c2826: 子会话头缺父会话 id"
    );
    assert!(
        cut.is_some_and(|c| !c.is_empty()),
        "c2826: 子会话头大切点条目 id"
    );
}

#[then("会话 {id:string} 头不含切点字段")]
async fn then_header_lacks_cut(sess: &XySessionStore, id: String) {
    let id = strip_quotes(&id);
    let header = first_disk_line(sess, &id);
    let cut = header
        .get("forkAtEntryId")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    assert!(
        cut.is_null(),
        "c2826: 非 fork 会话头 MUST NOT 写切点字段，实际为 {cut}"
    );
}

#[then("新会话磁盘头 version 为 7 且外壳字段为 camelCase")]
async fn then_header_v7_camelcase(sess: &XySessionStore) {
    let id = sess.current_id.borrow().clone().expect("current session");
    let header = first_disk_line(sess, &id);
    assert_eq!(header.get("version").and_then(|v| v.as_u64()), Some(7));
    let keys: Vec<&str> = header
        .as_object()
        .expect("header object")
        .keys()
        .map(String::as_str)
        .collect();
    assert!(
        keys.iter().all(|k| !k.contains('_')),
        "c2826: 头外壳字段必须 camelCase，出现 snake_case 键：{keys:?}"
    );
}

#[then("bash 记录行 type 为 message 且 role 为 bashExecution")]
async fn then_bash_row_shape(sess: &XySessionStore) {
    let id = sess.current_id.borrow().clone().expect("current session");
    let path = active_session_file(sess, &id);
    let raw = std::fs::read_to_string(&path).expect("active segment readable");
    let hit = raw
        .lines()
        .find(|l| l.contains("make test"))
        .expect("bash 记录行存在");
    let v: serde_json::Value = serde_json::from_str(hit).unwrap();
    assert_eq!(v.get("type").and_then(|t| t.as_str()), Some("message"));
    assert_eq!(
        v.get("message")
            .and_then(|m| m.get("role"))
            .and_then(|r| r.as_str()),
        Some("bashExecution")
    );
}

#[then("恢复投影模型为 {model:string} 且思考档为 {level:string} 且无抵消条目")]
async fn then_resume_projection_verbatim(sess: &XySessionStore, model: String, level: String) {
    let model = strip_quotes(&model);
    let level = strip_quotes(&level);
    let entries = sess.entries.borrow();
    let model_changes: Vec<&str> = entries
        .iter()
        .filter_map(|e| match e {
            SessionEntry::ModelChange(m) => Some(m.model_id.as_str()),
            _ => None,
        })
        .collect();
    let thinking_changes: Vec<&str> = entries
        .iter()
        .filter_map(|e| match e {
            SessionEntry::ThinkingLevelChange(t) => Some(t.thinking_level.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        model_changes,
        vec![model.as_str()],
        "c2826: leaf 投影模型必须原样保留且无抵消条目"
    );
    assert_eq!(
        thinking_changes,
        vec![level.as_str()],
        "c2826: leaf 投影思考档必须原样保留且无抵消条目"
    );
}
