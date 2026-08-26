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
}

#[fixture]
pub fn transcript_bdd() -> TranscriptBdd {
    TranscriptBdd {
        glyph_pair: RefCell::new(None),
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
