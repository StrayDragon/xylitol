use crate::bdd::prelude::*;
use rstest::fixture;
use rstest_bdd_macros::{then, when};

pub struct T4PrintBdd {
    pub out: RefCell<Vec<u8>>,
    pub err: RefCell<Vec<u8>>,
    pub result: RefCell<Option<Result<(), xylitol::XyDriverError>>>,
}

#[fixture]
pub fn t4_print_bdd() -> T4PrintBdd {
    T4PrintBdd {
        out: RefCell::new(Vec::new()),
        err: RefCell::new(Vec::new()),
        result: RefCell::new(None),
    }
}

pub(crate) fn t4_stream(
    events: Vec<xylitol_runtime::agent::runtime::XyEvent>,
) -> xylitol_app_tui::driver::EventStream {
    Box::pin(futures::stream::iter(events))
}

#[when("渲染 print 事件流到缓冲")]
pub(crate) async fn w_t4_render_text(t4_print_bdd: &T4PrintBdd) {
    use xylitol_plugin_sdk::message::{AgentMessage, AgentPart, XyLlmMessage};
    use xylitol_runtime::agent::runtime::XyEvent;
    let events = vec![
        XyEvent::MessageStart {
            role: "assistant".into(),
            message: None,
        },
        XyEvent::TextDelta("Hello".into()),
        XyEvent::MessageUpdate { message: None },
        XyEvent::TextDelta("!".into()),
        XyEvent::MessageEnd {
            role: "assistant".into(),
            message: Some(AgentMessage::Llm(XyLlmMessage::AssistantMessage {
                content: vec![AgentPart::text("Hello!")],
                stop_reason: None,
                usage: None,
                api: String::new(),
                provider: String::new(),
                model: String::new(),
                response_id: None,
                error_message: None,
                timestamp: 0,
                diagnostics: vec![],
            })),
        },
        XyEvent::AgentEnd {
            reason: None,
            messages: vec![],
        },
    ];
    let mut stream = t4_stream(events);
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let r = xylitol::cli::render_stream(&mut stream, &mut out, &mut err).await;
    *t4_print_bdd.out.borrow_mut() = out;
    *t4_print_bdd.result.borrow_mut() =
        Some(r.map_err(|e| xylitol::XyDriverError::message(e.to_string())));
}

#[then("stdout 恰为增量拼接且无前缀重复")]
pub(crate) fn t_t4_delta_only(t4_print_bdd: &T4PrintBdd) {
    let out = String::from_utf8(t4_print_bdd.out.borrow().clone()).unwrap();
    assert!(
        out.contains("Hello!"),
        "c2827: 增量应拼接为 Hello!：{out:?}"
    );
    assert!(
        !out.contains("HelloHello") && !out.contains("Hello! Hello! Hello"),
        "c2827: MessageUpdate 累积载荷 MUST NOT 写 stdout：{out:?}"
    );
}

#[when("渲染含工具执行的事件流到缓冲")]
pub(crate) async fn w_t4_render_tool(t4_print_bdd: &T4PrintBdd) {
    use xylitol_runtime::agent::runtime::XyEvent;
    let events = vec![
        XyEvent::ToolExecutionStart {
            id: "t1".into(),
            name: "read".into(),
            args: serde_json::json!({"path": "a.rs"}),
        },
        XyEvent::ToolExecutionEnd {
            id: "t1".into(),
            name: "read".into(),
            result: "file body".into(),
            details: None,
            is_error: false,
        },
        XyEvent::AgentEnd {
            reason: None,
            messages: vec![],
        },
    ];
    let mut stream = t4_stream(events);
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let r = xylitol::cli::render_stream(&mut stream, &mut out, &mut err).await;
    *t4_print_bdd.out.borrow_mut() = out;
    *t4_print_bdd.result.borrow_mut() =
        Some(r.map_err(|e| xylitol::XyDriverError::message(e.to_string())));
}

#[then("工具名与结果摘要按人话格式生成")]
pub(crate) fn t_t4_tool_display() {
    let start = xylitol::cli::format_tool_start_lines("read", &serde_json::json!({"path": "a.rs"}));
    assert!(
        start.iter().any(|l| l.contains("read")),
        "c2827: 工具开始行应含工具名：{start:?}"
    );
    let end = xylitol::cli::format_tool_end_line("read", "file body\nline2\nline3\nline4");
    assert!(
        end.contains("read") && end.contains("file body"),
        "c2827: 工具结束行应含名称与结果摘要：{end}"
    );
}

#[when("渲染含错误的事件流到缓冲")]
pub(crate) async fn w_t4_render_error(t4_print_bdd: &T4PrintBdd) {
    use xylitol_runtime::agent::runtime::XyEvent;
    let events = vec![
        XyEvent::TextDelta("partial".into()),
        XyEvent::Error(xylitol_plugin_sdk::lifecycle::XyEventError::new(
            "provider", "boom",
        )),
    ];
    let mut stream = t4_stream(events);
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let r = xylitol::cli::render_stream(&mut stream, &mut out, &mut err).await;
    *t4_print_bdd.out.borrow_mut() = out;
    *t4_print_bdd.result.borrow_mut() =
        Some(r.map_err(|e| xylitol::XyDriverError::message(e.to_string())));
}

#[then("错误返回驱动错误而工具单败不退出")]
pub(crate) fn t_t4_error_exit(t4_print_bdd: &T4PrintBdd) {
    let is_err = t4_print_bdd
        .result
        .borrow()
        .as_ref()
        .expect("result")
        .is_err();
    assert!(
        is_err,
        "c2827: Error 事件 MUST 返回驱动错误（进程非零退出）"
    );
}

#[when("渲染含单次工具失败的事件流到缓冲")]
pub(crate) async fn w_t4_render_tool_fail(t4_print_bdd: &T4PrintBdd) {
    use xylitol_runtime::agent::runtime::XyEvent;
    let events = vec![
        XyEvent::ToolExecutionEnd {
            id: "t1".into(),
            name: "bash".into(),
            result: "err".into(),
            details: None,
            is_error: true,
        },
        XyEvent::AgentEnd {
            reason: None,
            messages: vec![],
        },
    ];
    let mut stream = t4_stream(events);
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let r = xylitol::cli::render_stream(&mut stream, &mut out, &mut err).await;
    *t4_print_bdd.out.borrow_mut() = out;
    *t4_print_bdd.result.borrow_mut() =
        Some(r.map_err(|e| xylitol::XyDriverError::message(e.to_string())));
}

#[when("以含未闭合思考块的多消息流渲染 print 输出")]
pub(crate) async fn w_t4_render_message_interrupt(t4_print_bdd: &T4PrintBdd) {
    use xylitol_runtime::agent::runtime::XyEvent;
    let events = vec![
        XyEvent::MessageStart {
            role: "assistant".into(),
            message: None,
        },
        XyEvent::ThinkingDelta("first".into()),
        XyEvent::MessageStart {
            role: "assistant".into(),
            message: None,
        },
        XyEvent::ThinkingDelta("second".into()),
        XyEvent::TextDelta("ANSWER".into()),
        XyEvent::MessageEnd {
            role: "assistant".into(),
            message: None,
        },
    ];
    let mut stream = t4_stream(events);
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let r = xylitol::cli::render_stream(&mut stream, &mut out, &mut err).await;
    *t4_print_bdd.out.borrow_mut() = out;
    *t4_print_bdd.err.borrow_mut() = err;
    *t4_print_bdd.result.borrow_mut() =
        Some(r.map_err(|e| xylitol::XyDriverError::message(e.to_string())));
}

#[then("stderr 标签开闭配对且 stdout 仅正文")]
pub(crate) fn t_t4_thinking_interrupt_pairing(t4_print_bdd: &T4PrintBdd) {
    let out = String::from_utf8(t4_print_bdd.out.borrow().clone()).unwrap();
    assert!(!out.contains("<think>"), "stdout MUST 不含思考标签：{out}");
    assert!(out.contains("ANSWER"), "{out}");
    let err_text = String::from_utf8(t4_print_bdd.err.borrow().clone()).unwrap();
    assert_eq!(
        err_text.matches("<think>").count(),
        err_text.matches("</think>").count(),
        "开闭标签 MUST 配对：{err_text}"
    );
    assert!(
        err_text.matches("</think>").count() >= 1,
        "打断后旧块 MUST 已收口：{err_text}"
    );
}

#[when("以闭合标签跨片的思考流渲染 print 输出")]
pub(crate) async fn w_t4_render_split_close(t4_print_bdd: &T4PrintBdd) {
    use xylitol_runtime::agent::runtime::XyEvent;
    let events = vec![
        XyEvent::MessageStart {
            role: "assistant".into(),
            message: None,
        },
        XyEvent::ThinkingDelta("reasoning".into()),
        XyEvent::ThinkingDelta("</think>".into()),
        XyEvent::TextDelta("ANSWER".into()),
        XyEvent::MessageEnd {
            role: "assistant".into(),
            message: None,
        },
    ];
    let mut stream = t4_stream(events);
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let r = xylitol::cli::render_stream(&mut stream, &mut out, &mut err).await;
    *t4_print_bdd.out.borrow_mut() = out;
    *t4_print_bdd.err.borrow_mut() = err;
    *t4_print_bdd.result.borrow_mut() =
        Some(r.map_err(|e| xylitol::XyDriverError::message(e.to_string())));
}

#[then("stderr 闭合标签只出现一份")]
pub(crate) fn t_t4_thinking_split_close_single(t4_print_bdd: &T4PrintBdd) {
    let err_text = String::from_utf8(t4_print_bdd.err.borrow().clone()).unwrap();
    assert_eq!(
        err_text.matches("</think>").count(),
        1,
        "跨片闭合 MUST 只补一份：{err_text}"
    );
    assert_eq!(
        err_text.matches("<think>").count(),
        1,
        "补开前缀只此一份：{err_text}"
    );
}

#[then("单次工具失败不产生驱动错误")]
pub(crate) fn t_t4_tool_fail_ok(t4_print_bdd: &T4PrintBdd) {
    let ok = t4_print_bdd
        .result
        .borrow()
        .as_ref()
        .expect("result")
        .is_ok();
    assert!(ok, "c2827: 工具单次失败 MUST NOT 使 print 非零退出");
}
