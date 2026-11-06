use crate::bdd::prelude::*;
use rstest::fixture;
use rstest_bdd_macros::{given, then, when};
use serde_json::json;
use xylitol_app_tui::{UiEntry, UiModel, apply_xy_event};
use xylitol_plugin_sdk::XyEvent;

pub struct OutcomeBdd {
    pub model: RefCell<UiModel>,
    pub event: RefCell<Option<XyEvent>>,
}

#[fixture]
pub fn outcome_bdd() -> OutcomeBdd {
    OutcomeBdd {
        model: RefCell::new(UiModel::default()),
        event: RefCell::new(None),
    }
}

fn tool_end(details: Option<serde_json::Value>) -> XyEvent {
    XyEvent::ToolExecutionEnd {
        id: "rc1".into(),
        name: "run_code".into(),
        result: "42".into(),
        details,
        is_error: false,
    }
}

#[given("run_code 返回脚本结果与嵌套调用摘要")]
fn g_run_code_success(outcome_bdd: &OutcomeBdd) {
    *outcome_bdd.event.borrow_mut() = Some(tool_end(Some(json!({
        "script_summary": "read Cargo.toml and count packages",
        "elapsed_ms": 12,
        "truncated": false,
        "nested_calls": [
            {"name": "read", "args_preview": "Cargo.toml", "ok": true, "elapsed_ms": 3}
        ]
    }))));
}

#[when("工具执行结束事件到达")]
fn w_tool_end(outcome_bdd: &OutcomeBdd) {
    if let Some(event) = outcome_bdd.event.borrow().clone() {
        apply_xy_event(&mut outcome_bdd.model.borrow_mut(), &event);
    }
}

#[then("事件 details 含 script_summary 与 nested_calls")]
fn t_details_shape(outcome_bdd: &OutcomeBdd) {
    let event = outcome_bdd.event.borrow();
    let Some(XyEvent::ToolExecutionEnd {
        details: Some(details),
        ..
    }) = event.as_ref()
    else {
        panic!("expected ToolExecutionEnd with details");
    };
    assert!(details.get("script_summary").is_some());
    assert!(details.get("nested_calls").is_some());
}

#[given("run_code 执行超时")]
fn g_run_code_timeout(outcome_bdd: &OutcomeBdd) {
    *outcome_bdd.event.borrow_mut() = Some(tool_end(Some(json!({
        "error_kind": "timeout",
        "message": "run_code timed out"
    }))));
}

#[then("事件 details.error_kind 为 timeout 且保留可读错误")]
fn t_timeout_kind(outcome_bdd: &OutcomeBdd) {
    let event = outcome_bdd.event.borrow();
    let Some(XyEvent::ToolExecutionEnd {
        details: Some(details),
        ..
    }) = event.as_ref()
    else {
        panic!("expected ToolExecutionEnd with details");
    };
    assert_eq!(
        details.get("error_kind").and_then(|v| v.as_str()),
        Some("timeout")
    );
    assert!(details.get("message").and_then(|v| v.as_str()).is_some());
}

#[given("工具未提供 details")]
fn g_missing_details(outcome_bdd: &OutcomeBdd) {
    let mut model = outcome_bdd.model.borrow_mut();
    apply_xy_event(
        &mut model,
        &XyEvent::ToolExecutionStart {
            id: "rc2".into(),
            name: "run_code".into(),
            args: json!({"code": "return 42"}),
        },
    );
    *outcome_bdd.event.borrow_mut() = Some(XyEvent::ToolExecutionEnd {
        id: "rc2".into(),
        name: "run_code".into(),
        result: "plain result".into(),
        details: None,
        is_error: false,
    });
}

#[when("TUI 渲染工具结果")]
fn w_tui_render(outcome_bdd: &OutcomeBdd) {
    if let Some(event) = outcome_bdd.event.borrow().clone() {
        apply_xy_event(&mut outcome_bdd.model.borrow_mut(), &event);
    }
}

#[then("回退到文本且不伪造嵌套摘要")]
fn t_fallback(outcome_bdd: &OutcomeBdd) {
    let model = outcome_bdd.model.borrow();
    let output = model
        .entries
        .iter()
        .find_map(|entry| match entry {
            UiEntry::Tool { output, .. } => Some(output.as_str()),
            _ => None,
        })
        .expect("tool entry");
    assert!(output.contains("plain result"));
    assert!(!output.contains("nested calls"));
}
