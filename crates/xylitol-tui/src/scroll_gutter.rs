use crate::pointer::{Hit, HitId};
use crate::utils::ansi_escape_len;
use unicode_width::UnicodeWidthChar;

pub const GUTTER_MIN_WIDTH: usize = 20;

pub const GUTTER_TOP: char = '^';
pub const GUTTER_BOTTOM: char = 'v';
pub const GUTTER_TRACK: char = '│';
pub const GUTTER_THUMB: char = '┃';

pub const GUTTER_TOP_HIT_ID: HitId = 1 << 62;
pub const GUTTER_BOTTOM_HIT_ID: HitId = 1 << 62 | 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ScrollGutter;

#[must_use]
pub fn gutter_active(content_len: usize, viewport_h: usize, term_cols: usize) -> bool {
    content_len > viewport_h && viewport_h >= 2 && term_cols >= GUTTER_MIN_WIDTH
}

#[must_use]
pub fn gutter_inner_rows(viewport_h: usize) -> usize {
    viewport_h.saturating_sub(2)
}

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
            break;
        }
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
        let mut v = vec!["你".into()];
        let n = v.len();
        stamp_gutter(&mut v, 2, 0, n);
        assert_eq!(v[0], " ^");
        assert_eq!(width_of(&v[0]), 2);
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
