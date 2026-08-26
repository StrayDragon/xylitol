//! Steps for `app-tui-transcript` — P1 切片：纯字形合约（att19）。
//!
//! 帧级组装规则（scrollback 折叠状态机、鼠标命中等）待公共无头挂载面
//! （ScriptedDriver/UiRoot 导出，见分诊台账 P2 计划）后再转 executable。

use crate::prelude::*;
use rstest::fixture;
use rstest_bdd_macros::{given, then, when};
use xylitol::app::tui::GlyphSet;

/// Shared state for transcript glyph scenarios.
pub struct TranscriptBdd {
    pub glyph_pair: RefCell<Option<(&'static str, &'static str)>>,
    pub previews: RefCell<Vec<String>>,
    pub plain_frame: RefCell<Option<String>>,
}

#[fixture]
pub fn transcript_bdd() -> TranscriptBdd {
    TranscriptBdd {
        glyph_pair: RefCell::new(None),
        previews: RefCell::new(Vec::new()),
        plain_frame: RefCell::new(None),
    }
}

fn read_glyph_pair() -> (&'static str, &'static str) {
    let gs = GlyphSet::from_env();
    (gs.fold(), gs.unfold())
}

#[given("折叠字形环境未指定（默认 Unicode 集）")]
fn given_glyph_env_unset() {
    // SAFETY: 本变量仅本步骤文件读写；场景内串行翻转，无并发窗口。
    unsafe { std::env::remove_var("XYLITOL_TUI_GLYPH_SET") };
}

#[when("读取折叠与展开字形")]
fn when_read_glyphs(transcript_bdd: &TranscriptBdd) {
    transcript_bdd.glyph_pair.replace(Some(read_glyph_pair()));
}

#[then("折叠为 ▸ 展开为 ▾ 且各占单列")]
fn then_unicode_glyphs(transcript_bdd: &TranscriptBdd) {
    let pair = transcript_bdd.glyph_pair.borrow().expect("glyphs");
    assert_eq!(pair, ("▸", "▾"), "unicode fold/unfold glyphs");
    // att19：可视宽 1 列、无 ANSI 转义
    for g in [pair.0, pair.1] {
        assert_eq!(g.chars().count(), 1, "glyph must occupy 1 column: {g}");
        assert!(!g.contains('\x1b'), "glyph must not carry ANSI: {g}");
    }
}

#[then("切换环境变量 XYLITOL_TUI_GLYPH_SET=ascii 并重新读取")]
fn when_flip_ascii_reread(transcript_bdd: &TranscriptBdd) {
    // SAFETY: 同上；读后立即恢复删除，避免污染同进程其它用例。
    unsafe {
        std::env::set_var("XYLITOL_TUI_GLYPH_SET", "ascii");
    }
    let pair = read_glyph_pair();
    unsafe {
        std::env::remove_var("XYLITOL_TUI_GLYPH_SET");
    }
    transcript_bdd.glyph_pair.replace(Some(pair));
}

#[then("折叠回退为 > 展开回退为 v 且各占单列")]
fn then_ascii_fallback(transcript_bdd: &TranscriptBdd) {
    let pair = transcript_bdd.glyph_pair.borrow().expect("glyphs");
    assert_eq!(pair, (">", "v"), "ascii fold/unfold fallback glyphs");
}

// ---- att13：折叠态工具人话摘要（位置摘要，名字由 header 单独绘制）----

#[when("折叠态读取 bash、read、write 三类参数人话摘要")]
fn then_read_tool_previews(transcript_bdd: &TranscriptBdd) {
    use xylitol::app::tui::human_tool_args_preview;
    let bash = human_tool_args_preview("bash", &serde_json::json!({"command": "cargo test"}), 200);
    let read = human_tool_args_preview(
        "read",
        &serde_json::json!({"path": "src/lib.rs", "offset": 10, "limit": 5}),
        200,
    );
    let write = human_tool_args_preview("write", &serde_json::json!({"path": "docs/x.md"}), 200);
    *transcript_bdd.previews.borrow_mut() = vec![bash, read, write];
}

#[then("bash 前缀 $ 且 read 附行号区间且 write 为纯路径不带名前缀")]
fn then_tool_preview_shapes(transcript_bdd: &TranscriptBdd) {
    let previews = transcript_bdd.previews.borrow();
    assert_eq!(previews[0], "$ cargo test", "bash → $ {{command}}");
    assert_eq!(
        previews[1], "src/lib.rs:10-14",
        "read offset/limit → :start-end"
    );
    assert_eq!(previews[2], "docs/x.md", "write → 纯路径");
    for p in previews.iter() {
        assert!(
            !p.starts_with("bash ") && !p.starts_with("read ") && !p.starts_with("write "),
            "summary must be location-only (name painted separately): {p}"
        );
        assert!(!p.contains('{'), "must not leak raw args JSON: {p}");
    }
}

#[then("缺 path 时用三点占位且不回退完整 args JSON")]
fn then_missing_path_placeholder(transcript_bdd: &TranscriptBdd) {
    use xylitol::app::tui::human_tool_args_preview;
    // 前一步存的是三类正常摘要；这里直接补算 edit 无 path 场景
    let edit_no_path = human_tool_args_preview(
        "edit",
        &serde_json::json!({"edits": [{"oldText": "a", "newText": "b"}]}),
        200,
    );
    assert_eq!(edit_no_path, "...", "missing path must fall back to ...");
    assert!(
        !edit_no_path.contains("oldText"),
        "must not embed full args JSON: {edit_no_path}"
    );
}

// ---- att10：相邻块空行分隔（P2 无头帧挂载面首证）----

#[when("以场景构建器渲染相邻的助手块与工具块（宽 80）")]
fn then_render_adjacent_blocks(transcript_bdd: &TranscriptBdd) {
    use xylitol::app::tui::SceneBuilder;
    let mut sb = SceneBuilder::begin();
    sb.assistant("alpha body").message_end();
    sb.tool_start("t1", "read", "src/lib.rs")
        .tool_end("t1", "read");
    let (plain, _dump) = sb.render(80);
    *transcript_bdd.plain_frame.borrow_mut() = Some(plain);
}

#[then("相邻块之间至少一行空行分隔且不粘连成墙")]
fn then_block_gap_present(transcript_bdd: &TranscriptBdd) {
    let plain = transcript_bdd.plain_frame.borrow().clone().expect("frame");
    let lines: Vec<&str> = plain.lines().collect();
    let find_last = |needle: &str| {
        lines
            .iter()
            .rposition(|l| l.contains(needle))
            .unwrap_or_else(|| panic!("frame must contain {needle:?}:\n{plain}"))
    };
    let alpha = find_last("alpha body");
    let tool = find_last("lib.rs");
    assert!(
        tool > alpha,
        "tool block must render below assistant block:\n{plain}"
    );
    let gap_blank = lines[alpha + 1..tool].iter().any(|l| l.trim().is_empty());
    assert!(
        gap_blank,
        "att10: adjacent blocks need >=1 blank line between:\n{plain}"
    );
}
