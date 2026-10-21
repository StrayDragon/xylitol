use xylitol_tui::{
    DiffInput, DiffOptions, ExpandableOutputOptions, TruncateFrom, bold, render_diff_lines,
    render_expandable_output, visible_width,
};

use crate::layout::{LayoutTheme, TIER_WIDE_MIN_COLS};

use super::super::fold_hit::FoldTarget;
use super::cache::CachedFoldHit;
use super::paint::fit;

pub fn diff_fold_key(summary: &str, display_diff: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    summary.hash(&mut h);
    display_diff.hash(&mut h);
    format!("{:016x}", h.finish())
}

pub(super) const RAILED_MARKER_COL: usize = 2;

pub(super) const TOOLS_OUTPUT_PREVIEW_LINES: usize = 5;
pub(super) const WRITE_BODY_PREVIEW_LINES: usize = 10;
const DIFF_VIEWPORT_LINES: usize = 12;
const MAX_DIFF_RENDER_LINES: usize = 80;
const WORD_LEVEL_DIFF_LINE_LIMIT: usize = 120;

#[allow(clippy::too_many_arguments)]
pub(super) fn push_expandable_with_viewport_hit(
    lines: &mut Vec<String>,
    block_hits: &mut Vec<CachedFoldHit>,
    text: &str,
    width: usize,
    viewport_full: bool,
    opts: &ExpandableOutputOptions,
    col_offset: usize,
    target: FoldTarget,
) {
    let (out, footer_row) = render_expandable_output(text, width, viewport_full, opts);
    let base = lines.len();
    for line in &out {
        lines.push(fit(line, width));
    }
    if let Some(idx) = footer_row {
        let hint_cols = visible_width(&out[idx]).max(1);
        block_hits.push(CachedFoldHit {
            row_offset: base + idx,
            col_start: col_offset,
            col_end: col_offset.saturating_add(hint_cols),
            height: 1,
            target,
        });
    }
}

pub(super) fn push_viewport_diff_lines(
    lines: &mut Vec<String>,
    block_hits: &mut Vec<CachedFoldHit>,
    diff: &str,
    width: usize,
    theme: LayoutTheme,
    viewport_full: bool,
    target: FoldTarget,
) {
    let raw_lines = diff.lines().count();
    let word_level = raw_lines <= WORD_LEVEL_DIFF_LINE_LIMIT;
    let input = DiffInput::DisplayText(diff.to_string());
    let opts = DiffOptions {
        word_level,
        side_by_side_min_width: Some(TIER_WIDE_MIN_COLS),
        ..DiffOptions::default()
    };
    let surface = theme.palette().surface;
    let diff_theme = theme.palette().diff_theme_on_block(surface);
    let rendered = render_diff_lines(&input, width, &diff_theme, &opts);
    let body = if rendered.len() > MAX_DIFF_RENDER_LINES {
        let keep = MAX_DIFF_RENDER_LINES.saturating_sub(1);
        let omitted = rendered.len().saturating_sub(keep);
        let mut clipped: Vec<String> = rendered.into_iter().take(keep).collect();
        clipped.push(bold(&theme.paint_warning(&format!(
            "… ({omitted} more diff lines omitted — large edit capped for TUI)"
        ))));
        clipped.join("\n")
    } else {
        rendered.join("\n")
    };
    let exp_opts = ExpandableOutputOptions {
        max_preview_lines: DIFF_VIEWPORT_LINES,
        from: TruncateFrom::Tail,
        expand_hint: CTRL_O_EXPAND_HINT.into(),
        fold_hint: CTRL_O_FOLD_HINT.into(),
        hint_style: None,
    };
    push_expandable_with_viewport_hit(
        lines,
        block_hits,
        &body,
        width,
        viewport_full,
        &exp_opts,
        RAILED_MARKER_COL,
        target,
    );
}

pub(super) fn output_is_hard_truncated(output: &str) -> bool {
    output.lines().any(|l| l.starts_with("[Full output:"))
}

pub(super) const HARD_TRUNCATED_EXPAND_HINT: &str = "expand disabled — see Full output";

pub(super) const CTRL_O_EXPAND_HINT: &str = "ctrl+o to expand";
pub(super) const CTRL_O_FOLD_HINT: &str = "ctrl+o to fold";

pub(super) fn paint_output_with_full_footer(
    output: &str,
    theme: LayoutTheme,
    error: bool,
) -> String {
    let mut out = String::new();
    for (i, line) in output.lines().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        if line.starts_with("[Full output:") {
            out.push_str(&bold(&theme.paint_warning(line)));
        } else if error {
            out.push_str(&theme.paint_error(line));
        } else {
            out.push_str(&theme.paint_muted(line));
        }
    }
    if output.ends_with('\n') {
        out.push('\n');
    }
    out
}
