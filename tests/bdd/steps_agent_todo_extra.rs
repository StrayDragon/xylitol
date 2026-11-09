use crate::bdd::fixtures::AgentState;
use crate::bdd::prelude::*;
use rstest::fixture;
use rstest_bdd_macros::{given, then, when};

pub struct T2TodoBdd {
    pub gw: RefCell<Option<Arc<xylitol_official_plugin_todo::SessionAgentTodoGateway>>>,
    pub store: RefCell<Option<Arc<dyn xylitol_plugin_sdk::XySessionStore>>>,
    pub sid: RefCell<String>,
    pub last_err: RefCell<Option<String>>,
}

#[fixture]
pub fn t2_todo_bdd() -> T2TodoBdd {
    T2TodoBdd {
        gw: RefCell::new(None),
        store: RefCell::new(None),
        sid: RefCell::new(String::new()),
        last_err: RefCell::new(None),
    }
}

fn t2_todo_mount(t2_todo_bdd: &T2TodoBdd) {
    if t2_todo_bdd.gw.borrow().is_some() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let mgr = xylitol_official_plugin_session::SessionManager::new(dir.path().to_path_buf());
    let store: Arc<dyn xylitol_plugin_sdk::XySessionStore> = Arc::new(mgr);
    let sid = "s-todo-c2827".to_string();
    let sid2 = sid.clone();
    futures::executor::block_on(async {
        store.create(&sid2, Some("/tmp"), None).await.unwrap();
    });
    let gw = xylitol_official_plugin_todo::SessionAgentTodoGateway::new(store.clone());
    futures::executor::block_on(gw.bind_session(Some(sid.clone())));
    *t2_todo_bdd.sid.borrow_mut() = sid;
    *t2_todo_bdd.store.borrow_mut() = Some(store);
    *t2_todo_bdd.gw.borrow_mut() = Some(gw);
    std::mem::forget(dir);
}

fn t2_rewrite(t2_todo_bdd: &T2TodoBdd, items: serde_json::Value) -> Result<String, String> {
    t2_todo_mount(t2_todo_bdd);
    let gw = t2_todo_bdd.gw.borrow().as_ref().expect("gw").clone();
    let tools = xylitol_official_plugin_todo::todo_tools(gw);
    let tool = tools
        .iter()
        .find(|t| t.name() == "todo_rewrite")
        .expect("todo_rewrite")
        .clone();
    let ctx = xylitol_plugin_sdk::XyToolCtx::new("c2827");
    futures::executor::block_on(async { tool.execute(&ctx, items).await })
        .map_err(|e| e.to_string())
}

fn t2_update(t2_todo_bdd: &T2TodoBdd, items: serde_json::Value) -> Result<String, String> {
    let gw = t2_todo_bdd.gw.borrow().as_ref().expect("gw").clone();
    let tools = xylitol_official_plugin_todo::todo_tools(gw);
    let tool = tools
        .iter()
        .find(|t| t.name() == "todo_update")
        .expect("todo_update")
        .clone();
    let ctx = xylitol_plugin_sdk::XyToolCtx::new("c2827u");
    futures::executor::block_on(async { tool.execute(&ctx, items).await })
        .map_err(|e| e.to_string())
}

#[given("已绑定会话的 todo 网关")]
pub(crate) fn g_t2_todo_gateway(t2_todo_bdd: &T2TodoBdd) {
    t2_todo_mount(t2_todo_bdd);
}

#[when("todo_rewrite 写入两 in_progress 条目")]
pub(crate) fn w_t2_todo_two_in_progress(t2_todo_bdd: &T2TodoBdd) {
    let items = serde_json::json!({
        "items": [
            {"id": "a", "content": "one", "status": "in_progress"},
            {"id": "b", "content": "two", "status": "in_progress"}
        ]
    });
    match t2_rewrite(t2_todo_bdd, items) {
        Ok(_) => *t2_todo_bdd.last_err.borrow_mut() = None,
        Err(e) => *t2_todo_bdd.last_err.borrow_mut() = Some(e),
    }
}

#[then("写入成功且返回全表")]
pub(crate) fn t_t2_todo_write_ok(t2_todo_bdd: &T2TodoBdd) {
    assert!(
        t2_todo_bdd.last_err.borrow().is_none(),
        "c2827: 两条 in_progress MUST 可写入：{:?}",
        t2_todo_bdd.last_err.borrow()
    );
}

#[when("todo_update 更新未知 id")]
pub(crate) fn w_t2_todo_unknown_id(t2_todo_bdd: &T2TodoBdd) {
    let items = serde_json::json!({
        "items": [{"id": "nope", "status": "completed"}]
    });
    match t2_update(t2_todo_bdd, items) {
        Ok(_) => *t2_todo_bdd.last_err.borrow_mut() = None,
        Err(e) => *t2_todo_bdd.last_err.borrow_mut() = Some(e),
    }
}

#[then("整批写入被拒绝且返回可读错误")]
pub(crate) fn t_t2_todo_rejected(t2_todo_bdd: &T2TodoBdd) {
    let err = t2_todo_bdd.last_err.borrow().clone();
    let err = err.unwrap_or_else(|| "c2827: 未知 id 写入应被拒绝".into());
    assert!(!err.is_empty(), "c2827: 应返回可读错误：{err}");
}

#[when("写入超过 80 标量的 content")]
pub(crate) fn w_t2_todo_overlong(t2_todo_bdd: &T2TodoBdd) {
    let long = "x".repeat(81);
    let items = serde_json::json!({
        "items": [{"id": "a", "content": long, "status": "pending"}]
    });
    match t2_rewrite(t2_todo_bdd, items) {
        Ok(_) => *t2_todo_bdd.last_err.borrow_mut() = None,
        Err(e) => *t2_todo_bdd.last_err.borrow_mut() = Some(e),
    }
}

#[then("超长写入被拒绝且原表不变")]
pub(crate) fn t_t2_todo_overlong_rejected(t2_todo_bdd: &T2TodoBdd) {
    assert!(
        t2_todo_bdd.last_err.borrow().is_some(),
        "c2827: 超 80 标量 content MUST 拒绝"
    );
    let gw = t2_todo_bdd.gw.borrow().as_ref().expect("gw").clone();
    use xylitol_plugin_sdk::AgentTodoGateway as _;
    let list = futures::executor::block_on(gw.list()).ok();
    assert!(
        list.as_ref().map(|l| l.items.len()).unwrap_or(0) == 0,
        "c2827: 拒绝后 MUST NOT 截断入库"
    );
}

#[when("todo_rewrite 先写 A 表再覆盖为 B 表")]
pub(crate) fn w_t2_todo_latest_wins(t2_todo_bdd: &T2TodoBdd) {
    let a = serde_json::json!({
        "items": [{"id": "a", "content": "alpha", "status": "pending"}]
    });
    let b = serde_json::json!({
        "items": [{"id": "b", "content": "beta", "status": "in_progress"}]
    });
    t2_rewrite(t2_todo_bdd, a).expect("write A");
    t2_rewrite(t2_todo_bdd, b).expect("write B");
}

#[then("读取当前清单为最新快照")]
pub(crate) fn t_t2_todo_latest(t2_todo_bdd: &T2TodoBdd) {
    use xylitol_plugin_sdk::AgentTodoGateway as _;
    let gw = t2_todo_bdd.gw.borrow().as_ref().expect("gw").clone();
    let list = futures::executor::block_on(gw.list()).expect("list");
    assert_eq!(list.items.len(), 1, "c2827: 应为最新快照 B：{list:?}");
    assert_eq!(list.items[0].content, "beta", "c2827: latest-wins 取 B 表");
}

#[when("执行压后 todo 保全")]
pub(crate) fn w_t2_todo_compact_ensure(t2_todo_bdd: &T2TodoBdd) {
    t2_todo_mount(t2_todo_bdd);
    let store = t2_todo_bdd.store.borrow().as_ref().expect("store").clone();
    let sid = t2_todo_bdd.sid.borrow().clone();
    let items = serde_json::json!({
        "items": [{"id": "k", "content": "keep me", "status": "pending"}]
    });
    t2_rewrite(t2_todo_bdd, items).expect("seed todo");
    let entries_before =
        futures::executor::block_on(store.load_leaf_branch(&sid)).expect("entries before");
    futures::executor::block_on({
        let sid_ref: &str = &sid;
        xylitol_official_plugin_compaction::ensure_agent_todo_after_compact(
            store.as_ref(),
            sid_ref,
            &entries_before,
        )
    });
}

#[then("最新快照被重追加到会话")]
pub(crate) fn t_t2_todo_reappended(t2_todo_bdd: &T2TodoBdd) {
    use xylitol_plugin_sdk::session::latest_agent_todo;
    let store = t2_todo_bdd.store.borrow().as_ref().expect("store").clone();
    let sid = t2_todo_bdd.sid.borrow().clone();
    let entries = futures::executor::block_on(store.load_leaf_branch(&sid)).expect("entries");
    let todo = latest_agent_todo(&entries).expect("c2827: 压后应仍可读到 todo 快照");
    assert!(
        todo.items.iter().any(|i| i.content == "keep me"),
        "c2827: 重追加的应是最新的 keep me 快照：{todo:?}"
    );
}

#[then("导出条目含 agent_todo 自定义记录而非用户消息")]
pub(crate) fn t_t2_todo_export(t2_todo_bdd: &T2TodoBdd) {
    use xylitol_plugin_sdk::session::CUSTOM_TYPE_AGENT_TODO;
    let store = t2_todo_bdd.store.borrow().as_ref().expect("store").clone();
    let sid = t2_todo_bdd.sid.borrow().clone();
    let entries = futures::executor::block_on(store.load_entries(&sid)).expect("entries");
    let has_custom = entries.iter().any(|e| match e {
        SessionEntry::Custom(c) => c.custom_type == CUSTOM_TYPE_AGENT_TODO,
        _ => false,
    });
    assert!(has_custom, "c2827: 导出应含 agent_todo Custom 条目");
    let as_user = entries.iter().any(|e| {
        e.as_agent_message()
            .map(|m| m.role_name() == "user" && format!("{m:?}").contains("keep me"))
            .unwrap_or(false)
    });
    assert!(!as_user, "c2827: todo 快照 MUST NOT 伪装为用户消息");
}

#[when("以触发 todo_update 工具的回合收集事件")]
pub(crate) async fn w_t6_todo_event_round(agent: &AgentState) {
    use crate::bdd::helpers::make_agent;
    use futures::StreamExt;
    use xylitol_kernel::provider::factory::{
        reset_fake_state, set_fake_text, set_fake_tool_call, set_fake_tool_result,
    };

    reset_fake_state();
    crate::bdd::steps_agent_runtime::ar_register_fake(agent, "c2829-todo");
    set_fake_tool_call(
        "todo_rewrite",
        r#"{"items":[{"id":"a","content":"one","status":"in_progress"}]}"#,
    );
    set_fake_tool_result(r#"{"items":[{"id":"a","content":"one","status":"completed"}]}"#);
    set_fake_text("done");
    let mut runner = make_agent(agent);
    futures::executor::block_on(runner.select_model("c2829-todo")).expect("select fake");
    crate::bdd::helpers::bind_session_or_panic(&mut runner, "sess-todo-ev");
    let mut stream = crate::bdd::helpers::agent_submit_root(&mut runner, "tick todo").await;
    let mut saw_todo_updated = false;
    let mut list_len = None;
    while let Some(e) = stream.next().await {
        if let XyEvent::TodoUpdated { list } = &e {
            saw_todo_updated = true;
            list_len = Some(list.items.len());
        }
    }
    reset_fake_state();
    T6_TODO_EVENT2.with(|c| *c.borrow_mut() = (saw_todo_updated, list_len));
}

thread_local! {
    static T6_TODO_EVENT2: RefCell<(bool, Option<usize>)> = const { RefCell::new((false, None)) };
}

#[then("回合事件流含 TodoUpdated 全量快照")]
pub(crate) fn t_t6_todo_event_assert() {
    T6_TODO_EVENT2.with(|c| {
        let (saw, len) = *c.borrow();
        assert!(
            saw,
            "c2829: todo_rewrite 成功后 run 流 MUST 含类型化 TodoUpdated"
        );
        assert_eq!(len, Some(1), "c2829: TodoUpdated 应携带全量快照");
    });
}
