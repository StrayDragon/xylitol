//! Steps for `package-tui-markdown` (c2865 / r1936 partial fence trim).

use crate::bdd::prelude::*;
use rstest::fixture;
use rstest_bdd_macros::{then, when};
use xylitol_tui::utils::strip_ansi_codes;
use xylitol_tui::{Component, Markdown, Palette};

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
