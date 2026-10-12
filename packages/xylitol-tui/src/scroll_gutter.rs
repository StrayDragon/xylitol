//! Scroll gutter (c2891): transcript 右缘 1 列 `^ | v`（跳顶 / 轨道 / 跳底）。
//!
//! Opt-in（[`ScrollGutter`]，默认关）：开启时正文按少一列排版（宿主 carve，
//! 见 change design D2），本模块只负责末列 stamp 与可见性判定。
//! 隐藏三条件见 [`gutter_active`]——stamp / carve / hit 注册 MUST 共用同一判定。

use crate::pointer::{Hit, HitId};
use crate::utils::ansi_escape_len;
use unicode_width::UnicodeWidthChar;

/// 极窄阈值：可用宽低于此不占列（对齐 c2520 档位精神，窄屏不挤正文）。
pub const GUTTER_MIN_WIDTH: usize = 20;

/// 顶行跳顶记号。
pub const GUTTER_TOP: char = '^';
/// 底行跳底记号。
pub const GUTTER_BOTTOM: char = 'v';
/// 轨道字符（pi 同款；中段非 thumb 区）。
pub const GUTTER_TRACK: char = '│';
/// thumb 字符（拖拽中改粗的粗体观感由后续主题票承接入）。
pub const GUTTER_THUMB: char = '┃';

/// 跳顶 hit id（与产品 FoldHitTable 的 index+1 空间天然不相交）。
pub const GUTTER_TOP_HIT_ID: HitId = 1 << 62;
/// 跳底 hit id。
pub const GUTTER_BOTTOM_HIT_ID: HitId = 1 << 62 | 1;

/// 滚动槽开关（v1 无载荷；字符 / 颜色可后票扩展为字段）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ScrollGutter;

/// 滚动槽可见性：内容超屏 **且** 视口 ≥2 行（顶底异格）**且** 宽度达极窄阈值。
/// stamp / carve / hit 注册三方共用，不得各自复算。
#[must_use]
pub fn gutter_active(content_len: usize, viewport_h: usize, term_cols: usize) -> bool {
    content_len > viewport_h && viewport_h >= 2 && term_cols >= GUTTER_MIN_WIDTH
}

/// 可拖拽内域行数：排除顶 / 底按钮行（按钮恒占，thumb 域内缩）。
#[must_use]
pub fn gutter_inner_rows(viewport_h: usize) -> usize {
    viewport_h.saturating_sub(2)
}

/// thumb 高度：视口 / 全文比例，下限 2（pi `minThumbHeight`）且不超内域。
#[must_use]
pub fn gutter_thumb_len(content_len: usize, viewport_h: usize) -> usize {
    let inner = gutter_inner_rows(viewport_h);
    if inner == 0 || content_len == 0 {
        return 0;
    }
    let min_len = 2.min(inner);
    let proportional = ((inner * inner) / content_len).min(inner);
    min_len.max(proportional)
}

/// thumb 起始内域偏移（0 基，相对视口第 1 行）：滚动比例映射。
#[must_use]
pub fn gutter_thumb_offset(scroll_top: usize, content_len: usize, viewport_h: usize) -> usize {
    let inner = gutter_inner_rows(viewport_h);
    let thumb_len = gutter_thumb_len(content_len, viewport_h);
    if inner == 0 || thumb_len == 0 {
        return 0;
    }
    let max_scroll = content_len.saturating_sub(viewport_h);
    let max_thumb = inner - thumb_len;
    if max_scroll == 0 || max_thumb == 0 {
        return 0;
    }
    (scroll_top * max_thumb) / max_scroll
}

/// 反演：内域 pointer 行（0 基）+ grab → 目标 scroll_top（pi `scrollScrollbarToPointer`）。
#[must_use]
pub fn gutter_scroll_for_pointer(
    pointer_inner: usize,
    grab: usize,
    content_len: usize,
    viewport_h: usize,
) -> usize {
    let inner = gutter_inner_rows(viewport_h);
    let thumb_len = gutter_thumb_len(content_len, viewport_h);
    if inner == 0 || thumb_len == 0 {
        return 0;
    }
    let max_scroll = content_len.saturating_sub(viewport_h);
    let max_thumb = inner.saturating_sub(thumb_len);
    if max_thumb == 0 {
        return max_scroll;
    }
    let offset = pointer_inner.saturating_sub(grab).min(max_thumb);
    (offset * max_scroll) / max_thumb
}

/// 于 `visible` 每行右缘 `term_cols - 1` 列 stamp 记号：首行 top、末行 bottom、中间 track。
///
/// 行宽不足时先补普通空格（内容经 carve ≤ 该列，正常不覆盖；宽出则 ANSI 感知截断该列起尾部——零宽 SGR 保留）。调用方 MUST 已做过 [`gutter_active`] 判定。
pub fn stamp_gutter(
    visible: &mut [String],
    term_cols: usize,
    scroll_top: usize,
    content_len: usize,
) {
    let Some(gutter_col) = term_cols.checked_sub(1) else {
        return;
    };
    let rows = visible.len();
    if rows == 0 || gutter_col == 0 {
        return;
    }
    let thumb_start = 1 + gutter_thumb_offset(scroll_top, content_len, rows);
    let thumb_end = thumb_start + gutter_thumb_len(content_len, rows).min(rows.saturating_sub(2));
    for (i, line) in visible.iter_mut().enumerate() {
        let glyph = if i == 0 {
            GUTTER_TOP
        } else if i + 1 == rows {
            GUTTER_BOTTOM
        } else if i >= thumb_start && i < thumb_end {
            GUTTER_THUMB
        } else {
            GUTTER_TRACK
        };
        *line = set_col_glyph(line, gutter_col, glyph);
    }
}

/// 当前应注册的滚动槽命中集：active 时顶/底按钮两枚（非 capture，见上）；
/// 隐藏 / 未启用 / 内域不足时空。拖拽走 runtime 状态机，不经 hit。
#[must_use]
pub fn gutter_hit_set(term_cols: u16, transcript_rows: u16, active: bool) -> Vec<Hit> {
    if !active {
        return Vec::new();
    }
    let Some(col) = term_cols.checked_sub(1) else {
        return Vec::new();
    };
    if transcript_rows < 2 {
        return Vec::new();
    }
    let last_row = transcript_rows - 1;
    vec![
        Hit::at_cell(GUTTER_TOP_HIT_ID, col, 0),
        Hit::at_cell(GUTTER_BOTTOM_HIT_ID, col, last_row),
    ]
}

/// ANSI 感知地把 `col` 处显示格置为 `glyph`：不足补空格；跨越 `col` 的宽字符
/// 连同其后内容截除（carve 后不该发生，防御分支）。
fn set_col_glyph(line: &str, col: usize, glyph: char) -> String {
    let mut out = String::with_capacity(line.len() + 8);
    let mut width = 0usize;
    let mut chars = line.char_indices().peekable();
    while let Some((idx, ch)) = chars.next() {
        if let Some(len) = ansi_escape_len(line.as_bytes(), idx) {
            let seq = &line[idx..idx + len];
            out.push_str(seq);
            for _ in 1..len {
                chars.next();
            }
            continue;
        }
        let cw = ch.width().unwrap_or(0);
        let span_end = width + cw;
        if span_end <= col {
            out.push(ch);
            width = span_end;
            continue;
        }
        if width < col {
            // 宽字符跨越目标列：截到它之前（防御，carve 后不可达）。
            break;
        }
        // width > col：已越过目标列，丢弃余下正文（正常不可达）。
        break;
    }
    while width < col {
        out.push(' ');
        width += 1;
    }
    out.push(glyph);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn width_of(s: &str) -> usize {
        crate::utils::visible_width(s)
    }

    #[test]
    fn active_requires_overflow_two_rows_and_width() {
        assert!(gutter_active(9, 8, 40));
        assert!(!gutter_active(8, 8, 40), "content == viewport → hidden");
        assert!(
            !gutter_active(9, 1, 40),
            "viewport < 2 → top/bottom collide"
        );
        assert!(!gutter_active(9, 8, 19), "below min width");
    }

    #[test]
    fn stamps_top_thumb_track_bottom() {
        // 6 行视口：inner=4、thumb=2（顶部）→ ┃ 与 │ 并存，按钮恒占首尾
        let mut v: Vec<String> = ["a", "b", "c", "d", "e", "f"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let n = v.len();
        stamp_gutter(&mut v, 10, 0, n);
        let cols: Vec<_> = v.iter().map(|l| width_of(l)).collect();
        assert_eq!(cols, vec![10, 10, 10, 10, 10, 10]);
        assert!(v[0].ends_with('^'));
        assert!(v[1].ends_with('\u{2503}'), "thumb at head, got {:?}", v[1]);
        assert!(v[3].ends_with('\u{2502}'), "track below thumb");
        assert!(v[5].ends_with('v'));
        assert!(v[0].starts_with("a"), "content preserved before pad");
    }

    #[test]
    fn thumb_follows_scroll_ratio() {
        let mut v: Vec<String> = (0..6).map(|_| String::new()).collect();
        // content=20 vh=6: inner=4 thumb=2 max_thumb=2; scroll 12/12 → offset 2
        stamp_gutter(&mut v, 10, 12, 20);
        assert!(v[1].ends_with('\u{2502}'));
        assert!(v[3].ends_with('\u{2503}'), "thumb bottom at scroll end");
        assert!(v[5].ends_with('v'));
    }

    #[test]
    fn two_rows_skip_track() {
        let mut v = vec!["a".into(), "b".into()];
        let n = v.len();
        stamp_gutter(&mut v, 10, 0, n);
        assert!(v[0].ends_with('^'));
        assert!(v[1].ends_with('v'), "no track row between");
    }

    #[test]
    fn sgr_only_lines_keep_escapes_and_zero_width() {
        let mut v = vec!["\x1b[1mbold\x1b[0m".into(), "x".into()];
        let n = v.len();
        stamp_gutter(&mut v, 8, 0, n);
        assert!(v[0].ends_with('^'));
        assert!(v[0].contains("\x1b[1m"), "SGR preserved");
        assert_eq!(width_of(&v[0]), 8);
    }

    #[test]
    fn wide_char_spanning_gutter_col_is_cut_defensively() {
        // cols=2 → gutter_col=1 落在“你”（占 0..2）右半——防御性截除后补格。
        let mut v = vec!["你".into()];
        let n = v.len();
        stamp_gutter(&mut v, 2, 0, n);
        assert_eq!(v[0], " ^");
        assert_eq!(width_of(&v[0]), 2);
        // 对照：cols=3 → gutter_col=2，宽字符完整在列前，保留。
        let mut v = vec!["你".into()];
        let n = v.len();
        stamp_gutter(&mut v, 3, 0, n);
        assert_eq!(v[0], "你^");
    }

    #[test]
    fn hit_set_pins_to_screen_rows_across_scroll() {
        use crate::pointer::HitRegionList;
        let Some(top) = gutter_hit_set(40, 8, true).first().copied() else {
            panic!("active set must contain top hit");
        };
        let mut list = HitRegionList::new();
        for hit in gutter_hit_set(40, 8, true) {
            list.mark(hit);
        }
        // Viewport 语义：滚动前后同一屏幕格命中同一 id
        assert_eq!(list.hit_at(39, 0, 0, 8), Some(GUTTER_TOP_HIT_ID));
        assert_eq!(list.hit_at(39, 0, 12, 8), Some(GUTTER_TOP_HIT_ID));
        assert_eq!(list.hit_at(39, 7, 0, 8), Some(GUTTER_BOTTOM_HIT_ID));
        assert_eq!(list.hit_at(38, 0, 0, 8), None, "col 38 is content");
        let _ = top;
    }

    #[test]
    fn hits_are_uncaptured_and_hidden_set_empty() {
        for hit in gutter_hit_set(40, 8, true) {
            let region = hit.into_region();
            // capture 分支吞 pending_click，与 UpIfEmptySelection 不相容——必须非 capture
            assert!(!region.capturing);
            assert_eq!(region.height, 1);
        }
        assert!(gutter_hit_set(40, 8, false).is_empty());
        assert!(gutter_hit_set(40, 1, true).is_empty(), "rows < 2");
    }

    #[test]
    fn empty_rows_and_degenerate_width_are_noop_safe() {
        let mut v: Vec<String> = vec![String::new(), String::new()];
        let n = v.len();
        stamp_gutter(&mut v, 10, 0, n);
        assert!(v[0].ends_with('^'));
        assert!(v[1].ends_with('v'));
        let mut none: Vec<String> = vec![];
        stamp_gutter(&mut none, 10, 0, 0);
        assert!(none.is_empty());
        let mut tiny: Vec<String> = vec!["a".into()];
        stamp_gutter(&mut tiny, 1, 0, 1);
        assert_eq!(tiny[0], "a", "gutter_col == 0 → noop");
    }
}
