use xylitol_tui::{
    DiffInput, DiffOptions, ExpandableOutputOptions, TruncateFrom, bold, render_diff_lines,
    render_expandable_output, visible_width,
};

use crate::app::tui::layout::LayoutTheme;

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

/// Triangle column after left rail + gutter (`paint_left_rail_line`).
pub(super) const RAILED_MARKER_COL: usize = 2;

/// Max visual lines for collapsed tool/bash detail (pi bash tool = 5).
pub(super) const TOOLS_OUTPUT_PREVIEW_LINES: usize = 5;
/// Collapsed write body viewport (pi write.ts = 10 logical lines).
pub(super) const WRITE_BODY_PREVIEW_LINES: usize = 10;
/// Diff body viewport when Alt+E open but Ctrl+O not yet full.
const DIFF_VIEWPORT_LINES: usize = 12;
/// Max visual lines of edit/Diff body painted into scrollback (c1350).
const MAX_DIFF_RENDER_LINES: usize = 80;
/// Disable word-level when raw display_diff exceeds this many lines.
const WORD_LEVEL_DIFF_LINE_LIMIT: usize = 120;

/// Append expandable output; register the block's `OutputViewport` hit on the
/// visible Ctrl+O hint footer (collapsed) or fold-back footer (expanded,
/// peo3) — att30. `col_offset` is screen content col of the inner text
/// (2 when railed).
#[allow(clippy::too_many_arguments)] // text + viewport + hint geometry + fold target are distinct planes
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
    let out = render_expandable_output(text, width, viewport_full, opts);
    // The hint band is the last line whenever a footer exists: expand hint
    // footer when collapsed, fold footer when expanded past the viewport.
    let hint_idx = out
        .len()
        .checked_sub(1)
        .filter(|&idx| out[idx].contains(&opts.expand_hint) || out[idx].contains(&opts.fold_hint));
    let base = lines.len();
    for line in &out {
        lines.push(fit(line, width));
    }
    if let Some(idx) = hint_idx {
        let hint_cols = visible_width(&out[idx]).max(1);
        block_hits.push(CachedFoldHit {
            row_offset: base + idx,
            col_start: col_offset,
            col_end: col_offset.saturating_add(hint_cols),
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
        ..DiffOptions::default()
    };
    // Rail: no tool wash envelope; identity row bg via on_block(surface) so no diff-*-bg stack.
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

/// Hard system truncate (c1330/c1340): sidecar Full output footer present.
pub(super) fn output_is_hard_truncated(output: &str) -> bool {
    output.lines().any(|l| l.starts_with("[Full output:"))
}

pub(super) const HARD_TRUNCATED_EXPAND_HINT: &str = "expand disabled — see Full output";

/// Ctrl+O viewport expand hint — single literal shared by tool / bash / diff painters.
pub(super) const CTRL_O_EXPAND_HINT: &str = "ctrl+o to expand";
/// Expanded fold-back footer hint (peo3) — single literal shared by painters.
pub(super) const CTRL_O_FOLD_HINT: &str = "ctrl+o to fold";

/// Paint bash/tool body lines; Full output footer uses warning fg (att15 / pi).
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
