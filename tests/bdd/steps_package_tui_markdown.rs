use crate::bdd::prelude::*;
use rstest::fixture;
use rstest_bdd_macros::{then, when};
use xylitol_tui::utils::strip_ansi_codes;
use xylitol_tui::{Component, MERMAID_UNRENDERABLE, Markdown, MermaidFenceMode, Palette};

pub struct MarkdownFenceBdd {
    pub partial_len: Cell<usize>,
    pub complete_len: Cell<usize>,
    pub saw_fence_chrome: Cell<bool>,
    pub mismatch: RefCell<Option<String>>,
}

#[fixture]
pub fn markdown_fence_bdd() -> MarkdownFenceBdd {
    MarkdownFenceBdd {
        partial_len: Cell::new(0),
        complete_len: Cell::new(0),
        saw_fence_chrome: Cell::new(false),
        mismatch: RefCell::new(None),
    }
}

fn visible_code_block(src: &str) -> (usize, String) {
    let mut md = Markdown::new(
        src.to_string(),
        0,
        0,
        Palette::dark().markdown_theme(),
        None,
    );
    let vis: Vec<String> = md
        .render(40)
        .iter()
        .map(|l| strip_ansi_codes(l).trim_end().to_string())
        .collect();
    (vis.len(), vis.join("\n"))
}

fn has_fence_chrome(text: &str) -> bool {
    text.contains("```") || text.contains("~~~")
}

fn has_box_drawing(text: &str) -> bool {
    text.chars()
        .any(|c| ('\u{2500}'..='\u{257F}').contains(&c) || matches!(c, '▶' | '▼' | '◇'))
}

pub struct MarkdownMermaidBdd {
    pub visible: RefCell<String>,
}

#[fixture]
pub fn markdown_mermaid_bdd() -> MarkdownMermaidBdd {
    MarkdownMermaidBdd {
        visible: RefCell::new(String::new()),
    }
}

fn render_visible(src: &str, width: usize, mode: MermaidFenceMode) -> String {
    let mut md = Markdown::new(
        src.to_string(),
        0,
        0,
        Palette::dark().markdown_theme(),
        None,
    )
    .with_mermaid_mode(mode);
    md.render(width)
        .iter()
        .map(|l| strip_ansi_codes(l).trim_end().to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

#[when("渲染已闭合的最小 flowchart TD mermaid 围栏")]
fn w_closed_td(markdown_mermaid_bdd: &MarkdownMermaidBdd) {
    let src = "```mermaid\nflowchart TD\nA --> B\n```";
    *markdown_mermaid_bdd.visible.borrow_mut() = render_visible(src, 80, MermaidFenceMode::Final);
}

#[when("渲染已闭合但画不成盒线的 mermaid 围栏")]
fn w_garbage(markdown_mermaid_bdd: &MarkdownMermaidBdd) {
    let src = "```mermaid\nthis is not a diagram\n```";
    *markdown_mermaid_bdd.visible.borrow_mut() = render_visible(src, 80, MermaidFenceMode::Final);
}

#[when("以默认 final 渲染末尾未闭合的 mermaid 围栏")]
fn w_open_final(markdown_mermaid_bdd: &MarkdownMermaidBdd) {
    let src = "```mermaid\nflowchart TD\nA --> B";
    *markdown_mermaid_bdd.visible.borrow_mut() = render_visible(src, 80, MermaidFenceMode::Final);
}

#[when("以 off 模式渲染已闭合的最小 flowchart TD mermaid 围栏")]
fn w_off(markdown_mermaid_bdd: &MarkdownMermaidBdd) {
    let src = "```mermaid\nflowchart TD\nA --> B\n```";
    *markdown_mermaid_bdd.visible.borrow_mut() = render_visible(src, 80, MermaidFenceMode::Off);
}

#[when("以 streaming 模式渲染末尾未闭合且画不成盒线的 mermaid 围栏")]
fn w_streaming_garbage_open(markdown_mermaid_bdd: &MarkdownMermaidBdd) {
    let src = "```mermaid\nthis is not a diagram";
    *markdown_mermaid_bdd.visible.borrow_mut() =
        render_visible(src, 80, MermaidFenceMode::Streaming);
}

#[when("在窄于盒线自然宽的宽度渲染已闭合 mermaid 围栏")]
fn w_too_wide(markdown_mermaid_bdd: &MarkdownMermaidBdd) {
    let src = "```mermaid\nflowchart LR\nStart[Start Label] --> End[End Label]\n```";
    *markdown_mermaid_bdd.visible.borrow_mut() = render_visible(src, 12, MermaidFenceMode::Final);
}

fn leading_spaces(line: &str) -> usize {
    line.chars().take_while(|c| *c == ' ').count()
}

fn assert_fail_hint_then_indented_source(text: &str, body_needle: &str) {
    let hint_line = text
        .lines()
        .find(|l| l.contains(MERMAID_UNRENDERABLE))
        .unwrap_or_else(|| panic!("MUST 无法渲染提示：{text}"));
    let body_line = text
        .lines()
        .find(|l| l.contains(body_needle))
        .unwrap_or_else(|| panic!("MUST 回退代码块内源码：{text}"));
    let hint_at = text.find(MERMAID_UNRENDERABLE).expect("MUST 无法渲染提示");
    let body_at = text.find(body_needle).expect("MUST 回退代码块内源码");
    assert!(hint_at < body_at, "提示须在源码之前：\n{text}");
    assert!(
        leading_spaces(body_line) > leading_spaces(hint_line),
        "源码须比提示多一档缩进：\n{text}"
    );
}

#[then("可见行含盒线且无围栏铬")]
fn t_boxes_no_chrome(markdown_mermaid_bdd: &MarkdownMermaidBdd) {
    let text = markdown_mermaid_bdd.visible.borrow();
    assert!(has_box_drawing(&text), "MUST 输出盒线结构块：{text}");
    assert!(!has_fence_chrome(&text), "MUST NOT 围栏铬：{text}");
    assert!(
        !text.contains("flowchart TD"),
        "成功 MUST NOT 再输出围栏源码：{text}"
    );
    assert!(
        !text.contains(MERMAID_UNRENDERABLE),
        "成功 MUST NOT 无法渲染提示：{text}"
    );
}

#[then("可见行是源码且无盒线图")]
fn t_source_no_boxes(markdown_mermaid_bdd: &MarkdownMermaidBdd) {
    let text = markdown_mermaid_bdd.visible.borrow();
    assert!(
        text.contains("this is not a diagram")
            || text.contains("flowchart")
            || text.contains("Start"),
        "MUST 回退源码：{text}"
    );
    assert!(!has_box_drawing(&text), "MUST NOT 盒线图：{text}");
    assert!(!has_fence_chrome(&text), "MUST NOT 围栏铬：{text}");
    assert!(
        !text.contains(MERMAID_UNRENDERABLE),
        "off / 未闭合不是画失败：{text}"
    );
}

#[then("可见行是代码块内源码并带无法渲染提示且无盒线图")]
fn t_source_with_error_no_boxes(markdown_mermaid_bdd: &MarkdownMermaidBdd) {
    let text = markdown_mermaid_bdd.visible.borrow();
    assert_fail_hint_then_indented_source(&text, "this is not a diagram");
    assert!(!has_box_drawing(&text), "MUST NOT 盒线图：{text}");
    assert!(!has_fence_chrome(&text), "MUST NOT 围栏铬：{text}");
}

#[then("可见行是代码块内源码并带无法渲染提示且盒线未折行")]
fn t_source_with_error_no_wrapped_boxes(markdown_mermaid_bdd: &MarkdownMermaidBdd) {
    let text = markdown_mermaid_bdd.visible.borrow();
    let needle = if text.contains("flowchart") {
        "flowchart"
    } else {
        "Start"
    };
    assert_fail_hint_then_indented_source(&text, needle);
    assert!(!has_fence_chrome(&text), "MUST NOT 围栏铬：{text}");
    for line in text.lines() {
        assert!(!has_box_drawing(line), "MUST NOT 把盒线折行：{line}");
    }
}

#[when("渲染以半截闭合围栏结尾的围栏代码再补全该闭合围栏")]
fn w_partial_then_complete_fence(markdown_fence_bdd: &MarkdownFenceBdd) {
    let cases = [
        ("tick1", "```rust\nprint(1)\n", "`", "```"),
        ("tick2", "```rust\nprint(1)\n", "``", "```"),
        ("indent", "```rust\nprint(1)\n", "  ``", "```"),
        ("tilde", "~~~\nprint(1)\n", "~~", "~~~"),
    ];
    let mut chrome = false;
    for (label, body, partial, closer) in cases {
        let (partial_n, partial_text) = visible_code_block(&format!("{body}{partial}"));
        let (complete_n, complete_text) = visible_code_block(&format!("{body}{closer}"));
        markdown_fence_bdd.partial_len.set(partial_n);
        markdown_fence_bdd.complete_len.set(complete_n);
        chrome |= has_fence_chrome(&partial_text) || has_fence_chrome(&complete_text);
        if partial_n != complete_n {
            *markdown_fence_bdd.mismatch.borrow_mut() = Some(format!(
                "{label}: partial={partial_n} complete={complete_n}\n{partial_text}\n---\n{complete_text}"
            ));
            break;
        }
    }
    markdown_fence_bdd.saw_fence_chrome.set(chrome);
}

#[then("两次代码块可见行数相同且无围栏铬")]
fn t_fence_height_stable_no_chrome(markdown_fence_bdd: &MarkdownFenceBdd) {
    if let Some(msg) = markdown_fence_bdd.mismatch.borrow().as_deref() {
        panic!("半截 closer 与补全后代码块可见行数 MUST 相同：{msg}");
    }
    let partial = markdown_fence_bdd.partial_len.get();
    let complete = markdown_fence_bdd.complete_len.get();
    assert_eq!(
        partial, complete,
        "半截 closer 与补全后代码块可见行数 MUST 相同：partial={partial} complete={complete}"
    );
    assert!(
        !markdown_fence_bdd.saw_fence_chrome.get(),
        "MUST NOT 画出围栏铬或语言标签条"
    );
}
