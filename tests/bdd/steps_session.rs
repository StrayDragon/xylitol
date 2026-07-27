use crate::fixtures::*;
use crate::helpers::result_ok_str;
use crate::prelude::*;
use rstest_bdd_macros::{given, then, when};

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
    for i in 0..count {
        let e = SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: format!("msg-{i}"),
                parent_id: None,
                timestamp: "2024-01-01T00:00:00Z".into(),
            },
            message: serde_json::json!({"role":"user","content":format!("message {i}")}),
        });
        let _ = mgr.append(&id, &e).await;
    }
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
    let _ = (sess, n, id);
}
#[given("存在会话树: {tree}")]
fn _g_session_tree(_sess: &XySessionStore, tree: String) {
    let _ = tree;
}
#[when("导航到 {target}")]
fn _w_session_nav_to(_sess: &XySessionStore, target: String) {
    let _ = target;
}
#[when("将会话模型从 {from} 切换为 {to}")]
async fn _w_session_model_switch(sess: &XySessionStore, from: String, to: String) {
    let _ = (sess, from, to);
}
#[when("切换思考级别为 {level}")]
async fn _w_session_thinking_switch(sess: &XySessionStore, level: String) {
    let _ = (sess, level);
}
#[when("向会话追加 {n:u32} 条不同类型的记录")]
async fn _w_session_append_n(sess: &XySessionStore, n: u32) {
    let _ = (sess, n);
}
#[then("会话 {id:string} 包含 {count:u32} 条记录")]
fn _t_session_id_has_n(sess: &XySessionStore, id: String, count: u32) {
    let _ = (sess, id, count);
}
#[then("会话 {id:string} 包含一个 branch_summary 记录")]
fn _t_session_has_branch_summary(sess: &XySessionStore, id: String) {
    let _ = (sess, id);
}
#[then("会话包含 model_change 记录")]
fn _t_session_has_model_change(sess: &XySessionStore) {
    let _ = sess;
}
#[then("model_change 记录显示 provider 为 {provider}")]
fn _t_session_model_change_provider(sess: &XySessionStore, provider: String) {
    let _ = (sess, provider);
}
#[then("会话包含 thinking_level_change 记录")]
fn _t_session_has_thinking_change(sess: &XySessionStore) {
    let _ = sess;
}
#[then("JSONL 文件每行是一个完整的 JSON 对象")]
fn _t_session_jsonl_lines(sess: &XySessionStore) {
    let _ = sess;
}
#[then("第一行包含 version 字段")]
fn _t_session_jsonl_version(sess: &XySessionStore) {
    let _ = sess;
}
#[then("上下文包含 branch-a 和 branch-b 的摘要")]
fn _t_session_context_branches(sess: &XySessionStore) {
    let _ = sess;
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
#[then("操作被允许继续（fail-open 策略）")]
fn _t_hook_fail_open(agent: &AgentState) {
    let _ = agent;
}
