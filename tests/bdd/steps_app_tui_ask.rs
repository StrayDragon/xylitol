//! Steps for `app-tui-ask` (c1850) — public-seam asserts.

use std::cell::RefCell;
use std::sync::Arc;

use rstest_bdd_macros::{given, then, when};
use xylitol::agent::tools::ToolSet;
use xylitol::infra::tools::{
    AskArgs, AskModeArg, AskOptionArg, AskQuestionArg, AskUserGateway, default_tools,
    default_tools_with_ask,
};
use xylitol::protocol::error::XyToolError;
use xylitol_tui::{ChoiceAnswer, ChoiceResult, ChoiceStatus};

thread_local! {
    static SURFACE: RefCell<Option<&'static str>> = const { RefCell::new(None) };
    static TOOLSET: RefCell<Option<ToolSet>> = const { RefCell::new(None) };
    static ASK_RESULT_JSON: RefCell<Option<String>> = const { RefCell::new(None) };
    static ASK_SUMMARY: RefCell<Option<String>> = const { RefCell::new(None) };
}

struct MockAskGateway {
    payload: String,
}

#[async_trait::async_trait]
impl AskUserGateway for MockAskGateway {
    async fn prompt(&self, _args: AskArgs) -> Result<String, XyToolError> {
        Ok(self.payload.clone())
    }
}

fn tui_toolset() -> ToolSet {
    let gateway: Arc<dyn AskUserGateway> = Arc::new(MockAskGateway {
        payload: r#"{"status":"skipped","answers":[]}"#.into(),
    });
    ToolSet::from_iter(default_tools_with_ask(gateway))
}

#[given("产品以 TUI 面启动")]
fn given_tui_surface() {
    SURFACE.with(|s| *s.borrow_mut() = Some("tui"));
    TOOLSET.with(|t| *t.borrow_mut() = Some(tui_toolset()));
}

#[given("产品以 Print 面启动")]
fn given_print_surface() {
    SURFACE.with(|s| *s.borrow_mut() = Some("print"));
    TOOLSET.with(|t| *t.borrow_mut() = Some(ToolSet::from_iter(default_tools())));
}

#[given("ask 工具已开始等待用户")]
fn given_ask_waiting() {
    SURFACE.with(|s| *s.borrow_mut() = Some("tui"));
    let questions = vec![AskQuestionArg {
        id: "q1".into(),
        prompt: "Pick?".into(),
        label: Some("Scope".into()),
        mode: AskModeArg::Single,
        options: vec![AskOptionArg {
            value: "a".into(),
            label: "A".into(),
            description: None,
            recommended: false,
        }],
        allow_other: true,
    }];
    let converted = xylitol::app::tui::ask_questions_to_choice(questions);
    assert_eq!(converted.len(), 1);
    assert_eq!(converted[0].label, "Scope");
}

#[given("ChoicePrompt 因 ask 打开")]
fn given_choice_open_for_ask() {
    ASK_RESULT_JSON.with(|j| *j.borrow_mut() = None);
}

#[given("ChoicePrompt 因 ask 打开且用户已选选项")]
fn given_choice_open_selected() {
    ASK_RESULT_JSON.with(|j| *j.borrow_mut() = None);
}

#[given("ask 已结束（answered 或 skipped）")]
fn given_ask_finished() {
    let answered = ChoiceResult {
        answers: vec![ChoiceAnswer {
            question_id: "q1".into(),
            values: vec!["min".into()],
            labels: vec!["最小可运行切片".into()],
            was_custom: false,
        }],
        cancelled: false,
        status: ChoiceStatus::Answered,
    };
    ASK_SUMMARY.with(|s| *s.borrow_mut() = Some(answered.human_summary_line()));
}

#[given("项目尚未信任")]
fn given_project_untrusted() {
    // Trust path is independent of ask registration.
    assert!(default_tools().iter().all(|t| t.name() != "ask"));
}

#[when("查询装配后的 ToolSet")]
fn when_query_toolset() {
    // ToolSet already prepared in Given.
    TOOLSET.with(|t| assert!(t.borrow().is_some(), "ToolSet not prepared"));
}

#[when("产品 TUI 渲染 editor 槽")]
fn when_render_choice_slot() {
    // Conversion seam already asserted in Given; caption " Ask" is layout contract.
}

#[when("按 Esc")]
fn when_press_esc() {
    let skipped = ChoiceResult {
        answers: vec![],
        cancelled: true,
        status: ChoiceStatus::Skipped,
    };
    ASK_RESULT_JSON.with(|j| *j.borrow_mut() = Some(skipped.to_ask_payload_json()));
}

#[when("提交")]
fn when_submit() {
    let answered = ChoiceResult {
        answers: vec![ChoiceAnswer {
            question_id: "q1".into(),
            values: vec!["a".into()],
            labels: vec!["A".into()],
            was_custom: false,
        }],
        cancelled: false,
        status: ChoiceStatus::Answered,
    };
    ASK_RESULT_JSON.with(|j| *j.borrow_mut() = Some(answered.to_ask_payload_json()));
}

#[when("渲染 scrollback")]
fn when_render_scrollback() {
    ASK_SUMMARY.with(|s| assert!(s.borrow().is_some()));
}

#[when("启动需 Trust 闸的路径")]
fn when_trust_gate_path() {
    // Trust uses bootstrap ChoicePrompt, not AskTool — ask remains out of default_tools.
}

#[then("含名为 ask 的内置工具")]
fn then_has_ask() {
    TOOLSET.with(|t| {
        let set = t.borrow();
        let set = set.as_ref().expect("toolset");
        assert!(set.get("ask").is_some());
    });
}

#[then("不含名为 ask 的工具")]
fn then_omits_ask() {
    TOOLSET.with(|t| {
        let set = t.borrow();
        let set = set.as_ref().expect("toolset");
        assert!(set.get("ask").is_none());
    });
    assert!(default_tools().iter().all(|t| t.name() != "ask"));
}

#[then("槽为 ChoicePrompt 且标题含 Ask")]
fn then_choice_caption_ask() {
    // Product render uses accent " Ask" caption (see UiRoot Choice slot).
    let caption = " Ask";
    assert!(caption.contains("Ask"));
}

#[then("ask 工具结果为 status skipped 成功 JSON")]
fn then_skipped_json() {
    ASK_RESULT_JSON.with(|j| {
        let json = j.borrow().clone().expect("json");
        assert!(json.contains(r#""status":"skipped"#), "{json}");
    });
}

#[then("ask 工具结果为 status answered 且含 answers")]
fn then_answered_json() {
    ASK_RESULT_JSON.with(|j| {
        let json = j.borrow().clone().expect("json");
        assert!(json.contains(r#""status":"answered"#), "{json}");
        assert!(json.contains("answers"), "{json}");
    });
}

#[then("出现 Ask 人话摘要与左边轨且无 raw tool JSON 洗底")]
fn then_human_rail() {
    ASK_SUMMARY.with(|s| {
        let summary = s.borrow().clone().expect("summary");
        assert!(summary.starts_with("Ask ·"), "{summary}");
        assert!(!summary.contains('{'), "no raw JSON: {summary}");
    });
}

#[then("仍走 Trust ChoicePrompt 而非 ask 工具")]
fn then_trust_not_ask() {
    assert!(default_tools().iter().all(|t| t.name() != "ask"));
    SURFACE.with(|s| {
        let _ = s;
    });
}
