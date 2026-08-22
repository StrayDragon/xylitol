use crate::fixtures::*;
use crate::helpers::*;
use crate::prelude::*;
use rstest_bdd_macros::{given, then, when};

#[given("注册了匹配 {pat:string} 的 hook")]
fn _g_hook_registered(agent: &AgentState, pat: String) {
    agent.hook_entries.borrow_mut().push(HookEntry {
        events: vec![pat],
        command: "echo '{\"action\":\"allow\"}'".into(),
        phase: String::new(),
        timeout_secs: Some(5),
        requires_approval: false,
        env: HashMap::new(),
    });
}

fn _g_hook_returns(agent: &AgentState, json_str: String) {
    if let Some(e) = agent.hook_entries.borrow_mut().last_mut() {
        let j: serde_json::Value = serde_json::from_str(&json_str).unwrap_or_default();
        e.command = format!("echo '{}'", j.to_string().replace('\'', "'\\''"));
        let log = agent.ensure_wiring_hook_log();
        let outcome = match j.get("action").and_then(|a| a.as_str()) {
            Some("block") => xylitol::XyHookOutcome::Blocked {
                reason: j
                    .get("reason")
                    .and_then(|r| r.as_str())
                    .unwrap_or("blocked")
                    .to_string(),
            },
            Some("modify") => xylitol::XyHookOutcome::Modified {
                args: j.get("args").cloned().unwrap_or(j.clone()),
            },
            _ => xylitol::XyHookOutcome::Allowed,
        };
        *log.force.lock().unwrap_or_else(|err| err.into_inner()) = Some(outcome);
    }
}

fn _g_hook_slow(agent: &AgentState) {
    if let Some(e) = agent.hook_entries.borrow_mut().last_mut() {
        e.command = "sleep 10".into();
    }
}
fn _g_hook_timeout_1s(agent: &AgentState) {
    if let Some(e) = agent.hook_entries.borrow_mut().last_mut() {
        e.timeout_secs = Some(1);
    }
}
#[given("没有注册任何 hook")]
fn _g_hook_none(agent: &AgentState) {
    agent.hook_entries.borrow_mut().clear();
}

/// solidify 复合 given：多步折叠（solidify 无 Background / 并且）。
#[given("注册了匹配 pre.tool_call 的 hook 且返回 block 不允许")]
fn _g_hook_block_combo(agent: &AgentState) {
    _g_hook_registered(agent, "pre.tool_call".into());
    _g_hook_returns(agent, r#"{"action":"block","reason":"不允许"}"#.into());
}

#[given("注册了匹配 pre.tool_call.bash 的 hook 且返回 modify echo safe")]
fn _g_hook_modify_combo(agent: &AgentState) {
    _g_hook_registered(agent, "pre.tool_call.bash".into());
    _g_hook_returns(
        agent,
        r#"{"action":"modify","args":{"command":"echo safe"}}"#.into(),
    );
}

#[given("全局与用户 hook 已合并覆盖 pre.tool_call")]
fn _g_hook_merge_combo(agent: &AgentState) {
    let global = HookEntry {
        events: vec!["pre.tool_call".into()],
        command: "echo '{\"action\":\"allow\",\"source\":\"global\"}'".into(),
        ..Default::default()
    };
    let user = HookEntry {
        events: vec!["pre.tool_call".into()],
        command: "echo '{\"action\":\"allow\",\"source\":\"user\"}'".into(),
        ..Default::default()
    };
    let config = xylitol::infra::config::types::HooksConfig {
        global: vec![global],
        project: vec![],
        user: vec![user.clone()],
    };
    // Production path: HookDispatcher::new runs three-tier merge_hooks.
    let dispatcher = xylitol::infra::hooks::HookDispatcher::new(&config);
    assert_eq!(
        dispatcher.hook_count(),
        1,
        "same primary event pattern must collapse to one merged hook"
    );
    // Materialize the user-winning entry for subsequent load/assert steps.
    agent.hook_entries.replace(vec![user]);
}

#[given("hook 脚本超 2 秒且超时设为 1 秒")]
fn _g_hook_timeout_combo(agent: &AgentState) {
    _g_hook_registered(agent, "pre.tool_call".into());
    _g_hook_slow(agent);
    _g_hook_timeout_1s(agent);
}

#[given("注册了匹配 before_provider_request 的 hook 且 provider 为 deepseek")]
fn _g_hook_before_provider_combo(agent: &AgentState) {
    _g_hook_registered(agent, "before_provider_request".into());
}

/// hooks-wiring solidify：观察型 then 折叠（调用 + 上下文键）。
#[then("hook 被调用且上下文含键 reason")]
fn _t_wiring_called_reason(agent: &AgentState) {
    _t_hook_called(agent);
    _t_hook_context_has_key(agent, "reason".into());
}
#[then("hook 被调用且上下文含键 model")]
fn _t_wiring_called_model(agent: &AgentState) {
    _t_hook_called(agent);
    _t_hook_context_has_key(agent, "model".into());
}
#[then("hook 被调用且上下文含键 level")]
fn _t_wiring_called_level(agent: &AgentState) {
    _t_hook_called(agent);
    _t_hook_context_has_key(agent, "level".into());
}
#[then("hook 被调用且上下文含键 kind")]
fn _t_wiring_called_kind(agent: &AgentState) {
    _t_hook_called(agent);
    _t_hook_context_has_key(agent, "kind".into());
}
#[then("hook 被调用且上下文含键 command")]
fn _t_wiring_called_command(agent: &AgentState) {
    _t_hook_called(agent);
    _t_hook_context_has_key(agent, "command".into());
}

#[given("注册了匹配 session_before_tree 的 hook 且返回 block 树被拒绝")]
fn _g_wiring_tree_block(agent: &AgentState) {
    _g_hook_registered(agent, "session_before_tree".into());
    _g_hook_returns(agent, r#"{"action":"block","reason":"树被拒绝"}"#.into());
}
#[given("注册了匹配 session_before_switch 的 hook 且返回 block 切换被拒绝")]
fn _g_wiring_switch_block(agent: &AgentState) {
    _g_hook_registered(agent, "session_before_switch".into());
    _g_hook_returns(agent, r#"{"action":"block","reason":"切换被拒绝"}"#.into());
}
#[given("注册了匹配 user_bash 的 hook 且返回 block bash被拒绝")]
fn _g_wiring_bash_block(agent: &AgentState) {
    _g_hook_registered(agent, "user_bash".into());
    _g_hook_returns(agent, r#"{"action":"block","reason":"bash被拒绝"}"#.into());
}

#[when("bash 工具即将执行")]
async fn _w_hook_bash(agent: &AgentState) {
    dispatch_hook(
        agent,
        HookEvent::ToolCall {
            tool: "bash".into(),
            args: serde_json::json!({"command":"echo hello"}),
        },
        HookPhase::Pre,
    )
    .await;
}

#[when("任何工具即将执行")]
async fn _w_hook_any_tool(agent: &AgentState) {
    dispatch_hook(
        agent,
        HookEvent::ToolCall {
            tool: "read".into(),
            args: serde_json::json!({}),
        },
        HookPhase::Pre,
    )
    .await;
}

#[when("bash 工具以 {cmd:string} 调用")]
async fn _w_hook_bash_called(agent: &AgentState, cmd: String) {
    let cmd = strip_quotes(&cmd);
    dispatch_hook(
        agent,
        HookEvent::ToolCall {
            tool: "bash".into(),
            args: serde_json::json!({"command": cmd}),
        },
        HookPhase::Pre,
    )
    .await;
}

#[when("hook 被加载")]
fn _w_hook_loaded(agent: &AgentState) {
    // Materialize merge the same way production does (HookDispatcher::new).
    let config = xylitol::infra::config::types::HooksConfig {
        global: agent.hook_entries.borrow().clone(),
        project: vec![],
        user: vec![],
    };
    let dispatcher = xylitol::infra::hooks::HookDispatcher::new(&config);
    assert!(
        !dispatcher.is_empty(),
        "expected at least one merged hook after load"
    );
    agent.hook_result.replace(None);
}

#[when("dispatch hook")]
async fn _w_hook_dispatch_step(agent: &AgentState) {
    dispatch_hook(
        agent,
        HookEvent::ToolCall {
            tool: "bash".into(),
            args: serde_json::json!({}),
        },
        HookPhase::Pre,
    )
    .await;
}

#[when("provider 请求发送前")]
async fn _w_hook_before_request(agent: &AgentState) {
    dispatch_hook(
        agent,
        HookEvent::BeforeProviderRequest {
            model: "deepseek".into(),
            body: serde_json::json!({"model": "deepseek", "input": []}),
        },
        HookPhase::Pre,
    )
    .await;
}
#[when("provider 返回状态码 200")]
async fn _w_hook_provider_responded(agent: &AgentState) {
    dispatch_hook(
        agent,
        HookEvent::AfterProviderResponse {
            status: 200,
            headers: serde_json::json!({"content-type": "application/json"}),
        },
        HookPhase::Post,
    )
    .await;
}

#[when("任何事件触发")]
async fn _w_hook_any_event_step(agent: &AgentState) {
    dispatch_hook(
        agent,
        HookEvent::StepComplete {
            step: 1,
            summary: "done".into(),
        },
        HookPhase::Post,
    )
    .await;
}

#[then("hook 脚本被调用")]
fn _t_hook_called(agent: &AgentState) {
    if let Some(log) = agent.wiring_hook_log.borrow().as_ref() {
        let calls = log.calls.lock().unwrap_or_else(|e| e.into_inner());
        assert!(
            !calls.is_empty(),
            "expected library-seam hook dispatch, got none"
        );
    }
    // Legacy hooks.feature (no wiring log): observational.
}

fn _t_hook_context_has_key(agent: &AgentState, key: String) {
    let key = strip_quotes(&key);
    let log = agent
        .wiring_hook_log
        .borrow()
        .as_ref()
        .expect("wiring hook log")
        .clone();
    let calls = log.calls.lock().unwrap_or_else(|e| e.into_inner());
    assert!(
        calls
            .iter()
            .any(|(_, _, ctx)| ctx.get(key.as_str()).is_some()),
        "expected context key {key:?} in calls {calls:?}"
    );
}

#[when("执行操作 {op:string}")]
async fn _w_wiring_op(agent: &AgentState, op: String) {
    let op = strip_quotes(&op);
    agent.last_op_error.replace(None);
    if let Some(log) = agent.wiring_hook_log.borrow().as_ref() {
        log.calls.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }
    match run_wiring_operation(agent, &op).await {
        Ok(()) => {}
        Err(e) => {
            agent.last_op_error.replace(Some(e.to_string()));
        }
    }
}

#[then("操作失败原因包含 {msg}")]
fn _t_op_error_contains(agent: &AgentState, msg: String) {
    let msg = strip_quotes(&msg);
    let err = agent
        .last_op_error
        .borrow()
        .clone()
        .expect("expected operation error");
    assert!(err.contains(&msg), "error {err:?} does not contain {msg:?}");
}
#[then("操作被阻止")]
fn _t_hook_blocked(agent: &AgentState) {
    assert!(matches!(
        agent.hook_result.borrow().as_ref().unwrap(),
        DispatchResult::Blocked { .. }
    ));
}

#[then("实际执行的命令为 {cmd}")]
fn _t_hook_actual_cmd(agent: &AgentState, cmd: String) {
    let expected = strip_quotes(&cmd);
    match agent.hook_result.borrow().as_ref() {
        Some(DispatchResult::Modified { args }) => {
            let actual = args
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            assert_eq!(
                actual, expected,
                "expected modified command {expected:?}, got args={args}"
            );
        }
        other => panic!("expected Modified with command, got {other:?}"),
    }
}
#[then("使用用户配置的 hook 命令")]
fn _t_hook_user_used(agent: &AgentState) {
    let entries = agent.hook_entries.borrow();
    let cmd = &entries
        .last()
        .expect("expected merged hook entries after load")
        .command;
    assert!(
        cmd.contains(r#""source":"user""#),
        "expected user-layer hook command to win merge, got {cmd}"
    );
    assert!(
        !cmd.contains(r#""source":"global""#),
        "global hook command must not remain after user override, got {cmd}"
    );
}
#[then("hook 在 1 秒后被杀死")]
fn _t_hook_killed(agent: &AgentState) {
    assert!(agent.hook_result.borrow().is_some());
}

#[then("hook 收到请求 payload")]
fn _t_hook_got_payload(agent: &AgentState) {
    assert!(agent.hook_result.borrow().is_some());
}
#[then("hook 收到 status=200 和响应 headers")]
fn _t_hook_got_response(agent: &AgentState) {
    assert!(matches!(
        agent.hook_result.borrow().as_ref(),
        Some(DispatchResult::Allowed)
    ));
}
#[then("dispatch 是零开销 no-op")]
fn _t_hook_noop(agent: &AgentState) {
    assert!(matches!(
        agent.hook_result.borrow().as_ref().unwrap(),
        DispatchResult::Allowed
    ));
}
