use xylitol_tui::terminal_colors::RgbColor;
use xylitol_tui::{
    Component, ExpandableOutputOptions, Markdown, TruncateFrom, highlight_dollar_skill_refs,
    mix_rgb, paint_left_rail_line, truncate_to_width, visible_width, wrap_text_with_ansi,
};

use crate::app::tui::activity_fold::{format_elapsed_secs, thought_header_body};
use crate::app::tui::bridge::{AskPhase, BashBlockStatus, CompactionBlockStatus};
use crate::app::tui::layout::LayoutTheme;

use super::super::fold_hit::FoldTarget;
use super::super::glyphs::GlyphSet;
use super::ScrollbackFold;
use super::cache::{CachedFoldHit, marker_cols};
use super::diff::{
    CTRL_O_EXPAND_HINT, HARD_TRUNCATED_EXPAND_HINT, RAILED_MARKER_COL, TOOLS_OUTPUT_PREVIEW_LINES,
    WRITE_BODY_PREVIEW_LINES, diff_fold_key, output_is_hard_truncated,
    paint_output_with_full_footer, push_expandable_with_viewport_hit, push_viewport_diff_lines,
};

pub(super) fn key_hint(chord: &str) -> String {
    format!("({chord})")
}

/// Ask scrollback header: accent **Ask** + ellipsized ` · q → a · …` rest; fits `inner`.
pub(super) fn paint_ask_header_line(
    theme: LayoutTheme,
    marker: &str,
    summary: &str,
    inner: usize,
) -> String {
    let hint = theme.paint_muted(&key_hint("Alt+E"));
    let ask = theme.paint_tool_name("Ask");
    let rest = summary
        .strip_prefix("Ask")
        .map(str::to_string)
        .unwrap_or_else(|| {
            if summary.is_empty() {
                String::new()
            } else {
                format!(" · {summary}")
            }
        });
    // `{marker} {Ask}{rest}  {hint}`
    let fixed =
        visible_width(marker) + 1 + visible_width("Ask") + 2 + visible_width(&key_hint("Alt+E"));
    let rest_budget = inner.saturating_sub(fixed).max(4);
    let rest_fit = if visible_width(&rest) <= rest_budget {
        rest
    } else {
        truncate_to_width(&rest, rest_budget, "…", false)
    };
    format!("{marker} {ask}{rest_fit}  {hint}")
}

/// Format token counts with thousands separators (pi / design fixture).
pub(super) fn format_token_count(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out.chars().rev().collect()
}

pub(super) fn paint_tool_header_line(
    theme: LayoutTheme,
    marker: &str,
    name: &str,
    args_preview: &str,
    timeout_secs: Option<u64>,
) -> String {
    use crate::app::tui::bridge::display_tool_title;
    let title = display_tool_title(name);
    let hint = theme.paint_muted(&key_hint("Alt+E"));
    // c2435: budget note only when the model explicitly requested a bound.
    let timeout_note = match timeout_secs {
        Some(n) => theme.paint_muted(&format!("(timeout {n}s) ")),
        None => String::new(),
    };
    if args_preview.is_empty() {
        format!(
            "{marker} {}  {timeout_note}{hint}",
            theme.paint_tool_name(&title)
        )
    } else {
        let (path, range) = split_path_and_range(args_preview);
        let loc = match range {
            Some(r) => format!(
                "{}{}",
                theme.paint_tool_path(path),
                theme.paint_tool_range(r)
            ),
            None => theme.paint_tool_path(path),
        };
        format!(
            "{marker} {} {}  {timeout_note}{hint}",
            theme.paint_tool_name(&title),
            loc
        )
    }
}

/// Split `path:12-40` / `path:42:8` — range suffix painted separately (warning).
fn split_path_and_range(loc: &str) -> (&str, Option<&str>) {
    if loc.starts_with('$') {
        return (loc, None);
    }
    for (i, ch) in loc.char_indices().rev() {
        if ch != ':' {
            continue;
        }
        let suffix = &loc[i..];
        if is_line_range_suffix(suffix) {
            return (&loc[..i], Some(suffix));
        }
    }
    (loc, None)
}

fn is_line_range_suffix(s: &str) -> bool {
    let Some(body) = s.strip_prefix(':') else {
        return false;
    };
    if body.is_empty() || !body.as_bytes()[0].is_ascii_digit() {
        return false;
    }
    // :N | :N-M | :N:C | :N-M:C (C optional col — rare)
    let mut saw_digit = false;
    let mut seps = 0u8;
    for b in body.bytes() {
        if b.is_ascii_digit() {
            saw_digit = true;
            continue;
        }
        if (b == b'-' || b == b':') && saw_digit && seps < 2 {
            seps += 1;
            saw_digit = false;
            continue;
        }
        return false;
    }
    saw_digit
}

pub(super) fn fit(text: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let clipped = if visible_width(text) > width {
        truncate_to_width(text, width, "...", false)
    } else {
        text.to_string()
    };
    let pad = width.saturating_sub(visible_width(&clipped));
    format!("{clipped}{}", " ".repeat(pad))
}

pub(super) fn push_wrapped(lines: &mut Vec<String>, raw: &str, width: usize) {
    for line in wrap_text_with_ansi(raw, width.max(1)) {
        lines.push(fit(&line, width));
    }
}

/// Untinted full-width row between blocks (pi `Spacer(1)`).
///
/// Must be spaces + `\x1b[49m` — a bare `""` does not reliably occupy a visible
/// terminal row after differential clear, so gaps looked missing.
pub(super) fn inter_block_spacer(width: usize) -> String {
    format!("{}\x1b[49m", " ".repeat(width.max(1)))
}

/// Content width inside a railed block (rail 1 + gutter 1).
pub(super) fn rail_inner_width(width: usize) -> usize {
    width.saturating_sub(2).max(1)
}

/// Status rail + gutter for tool/thinking/bash/diff blocks (c1830).
pub(super) fn push_railed(
    lines: &mut Vec<String>,
    content: &[String],
    width: usize,
    rgb: RgbColor,
) {
    for line in content {
        lines.push(paint_left_rail_line(line, width, rgb));
    }
}

pub(super) fn tool_rail_rgb(pending: bool, is_error: bool, theme: LayoutTheme) -> RgbColor {
    let p = theme.palette();
    let vivid = if pending {
        p.accent
    } else if is_error {
        p.error
    } else {
        p.success
    };
    mix_rgb(p.surface, vivid, 0.72)
}

pub(super) fn bash_rail_rgb(status: BashBlockStatus, theme: LayoutTheme) -> RgbColor {
    let p = theme.palette();
    let vivid = match status {
        BashBlockStatus::Pending => p.accent,
        BashBlockStatus::Success => p.success,
        BashBlockStatus::Error | BashBlockStatus::Cancelled => p.error,
    };
    mix_rgb(p.surface, vivid, 0.72)
}

pub(super) fn ask_rail_rgb(phase: AskPhase, theme: LayoutTheme) -> RgbColor {
    let p = theme.palette();
    let vivid = match phase {
        AskPhase::Waiting => p.accent,
        AskPhase::Answered => p.success,
        AskPhase::Skipped => p.muted,
    };
    mix_rgb(p.surface, vivid, 0.72)
}

/// End index of a stable markdown prefix (after last `\n\n` not inside a fence).
///
/// Streaming paint reuses lines for `text[..end]` and only re-parses the suffix.
pub fn find_stable_markdown_prefix_end(text: &str) -> usize {
    let bytes = text.as_bytes();
    let mut in_fence = false;
    let mut last_stable = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        let at_line_start = i == 0 || bytes[i - 1] == b'\n';
        if at_line_start && bytes[i] == b'`' {
            let mut j = i;
            while j < bytes.len() && bytes[j] == b'`' {
                j += 1;
            }
            if j - i >= 3 {
                in_fence = !in_fence;
                i = j;
                continue;
            }
        }
        if !in_fence && bytes[i] == b'\n' && i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
            last_stable = i + 2;
            i += 2;
            continue;
        }
        i += 1;
    }
    last_stable
}

/// Incremental paint cache for `streaming_assistant` (ath26 / c1510).
#[derive(Debug, Default)]
pub struct StreamingAssistantPaint {
    width: usize,
    stable_prefix: String,
    prefix_lines: Vec<String>,
    /// Full-buffer Markdown parses (no prefix reuse).
    pub(crate) full_parses: u64,
    /// Suffix-only (or stable-extend) Markdown parses.
    pub(crate) suffix_parses: u64,
}

impl StreamingAssistantPaint {
    pub fn invalidate(&mut self) {
        self.width = 0;
        self.stable_prefix.clear();
        self.prefix_lines.clear();
    }

    #[cfg(test)]
    pub fn clear_counts(&mut self) {
        self.full_parses = 0;
        self.suffix_parses = 0;
    }

    fn prepare_width(&mut self, width: usize) {
        if self.width != width {
            self.invalidate();
            self.width = width;
        }
    }
}

fn markdown_fit_lines(text: String, width: usize, theme: LayoutTheme) -> Vec<String> {
    let mut md = Markdown::new(text, 0, 0, theme.palette().markdown_theme(), None);
    md.render(width)
        .into_iter()
        .map(|line| fit(&line, width))
        .collect()
}

pub(super) fn paint_streaming_assistant(
    text: &str,
    width: usize,
    theme: LayoutTheme,
    stream: &mut StreamingAssistantPaint,
) -> Vec<String> {
    let width = width.max(1);
    stream.prepare_width(width);
    if text.is_empty() {
        stream.invalidate();
        stream.width = width;
        return markdown_fit_lines("…".into(), width, theme);
    }

    let stable_end = find_stable_markdown_prefix_end(text);
    let new_stable = &text[..stable_end];

    if !stream.stable_prefix.is_empty()
        && text.starts_with(stream.stable_prefix.as_str())
        && new_stable.starts_with(stream.stable_prefix.as_str())
    {
        if new_stable.len() > stream.stable_prefix.len() {
            let chunk = &text[stream.stable_prefix.len()..stable_end];
            stream.suffix_parses = stream.suffix_parses.saturating_add(1);
            stream
                .prefix_lines
                .extend(markdown_fit_lines(chunk.to_string(), width, theme));
            stream.stable_prefix = new_stable.to_string();
        }
        stream.suffix_parses = stream.suffix_parses.saturating_add(1);
        let mut out = stream.prefix_lines.clone();
        out.extend(markdown_fit_lines(
            format!("{}…", &text[stable_end..]),
            width,
            theme,
        ));
        return out;
    }

    stream.full_parses = stream.full_parses.saturating_add(1);
    let all = markdown_fit_lines(format!("{text}…"), width, theme);
    if stable_end > 0 {
        // Seed prefix cache for subsequent deltas (extra parse; not counted as full).
        stream.prefix_lines = markdown_fit_lines(new_stable.to_string(), width, theme);
        stream.stable_prefix = new_stable.to_string();
    } else {
        stream.prefix_lines.clear();
        stream.stable_prefix.clear();
    }
    all
}

// ── Per-`UiEntry` block painters (extracted from `render_scrollback`) ────────
//
// Each paints one entry into block lines + fold hits; the caller owns the
// paint cache (fingerprint hit / miss) and fold-hit emission.

/// Shared paint-plane inputs (fold toggles, glyphs, theme, width).
#[derive(Clone, Copy)]
pub(super) struct PaintCtx<'a> {
    pub fold: &'a ScrollbackFold,
    pub glyphs: GlyphSet,
    pub theme: LayoutTheme,
    pub width: usize,
}

/// User row: `prefix text` — no user-message-bg wash, no status rail (atc8 / c1830).
pub(super) fn paint_user_block(text: &str, ctx: PaintCtx) -> (Vec<String>, Vec<CachedFoldHit>) {
    let PaintCtx {
        glyphs,
        theme,
        width,
        ..
    } = ctx;
    let mut lines = Vec::new();
    let prefix = theme.paint_user(glyphs.user());
    let painted = highlight_dollar_skill_refs(text, theme.palette().skill_ref);
    push_wrapped(&mut lines, &format!("{prefix} {painted}"), width);
    (lines, Vec::new())
}

/// Assistant row: Markdown render + fit (no rail).
pub(super) fn paint_assistant_block(
    text: &str,
    ctx: PaintCtx,
) -> (Vec<String>, Vec<CachedFoldHit>) {
    let PaintCtx { theme, width, .. } = ctx;
    let mut lines = Vec::new();
    let mut md = Markdown::new(
        text.to_string(),
        0,
        0,
        theme.palette().markdown_theme(),
        None,
    );
    for line in md.render(width) {
        lines.push(fit(&line, width));
    }
    (lines, Vec::new())
}

/// Thinking row: foldable header + optional body. `thought_only` entries fold
/// into the cluster header (no second L1 row).
pub(super) fn paint_thinking_block(
    id: &str,
    text: &str,
    elapsed_secs: Option<u64>,
    thought_only: bool,
    ctx: PaintCtx,
) -> (Vec<String>, Vec<CachedFoldHit>) {
    let PaintCtx {
        fold,
        glyphs,
        theme,
        width,
    } = ctx;
    let mut lines = Vec::new();
    let mut block_hits = Vec::new();
    if thought_only {
        // Cluster header is already Thought; don't paint a second L1 row.
        push_wrapped(&mut lines, &theme.paint_muted(text), width);
        return (lines, block_hits);
    }
    let expanded = fold.thinking_effective(id);
    let marker = if expanded {
        glyphs.unfold()
    } else {
        glyphs.fold()
    };
    let mw = marker_cols(marker);
    let dur = elapsed_secs.filter(|s| *s > 0).map(format_elapsed_secs);
    let label = thought_header_body(dur.as_deref());
    let header = theme.paint_muted(&format!("{marker} {label}  {}", key_hint("Ctrl+T")));
    let header_row = lines.len();
    push_wrapped(&mut lines, &header, width);
    block_hits.push(CachedFoldHit {
        row_offset: header_row,
        col_start: 0,
        col_end: mw,
        target: FoldTarget::Thinking(id.to_string()),
    });
    if expanded {
        push_wrapped(&mut lines, &theme.paint_muted(text), width);
    }
    (lines, block_hits)
}

/// Tool row: railed header + optional write body / output / diff (c1300).
#[allow(clippy::too_many_arguments)] // tool row carries the richest payload (distinct paint plane)
pub(super) fn paint_tool_block(
    id: &str,
    name: &str,
    args_preview: &str,
    timeout_secs: Option<u64>,
    write_content: Option<&str>,
    display_diff: Option<&str>,
    output: &str,
    is_error: bool,
    done: bool,
    ctx: PaintCtx,
) -> (Vec<String>, Vec<CachedFoldHit>) {
    let PaintCtx {
        fold,
        glyphs,
        theme,
        width,
    } = ctx;
    let inner = rail_inner_width(width);
    let expanded = fold.tools_effective(id);
    let marker = if expanded {
        glyphs.unfold()
    } else {
        glyphs.fold()
    };
    let mw = marker_cols(marker);
    let header = paint_tool_header_line(theme, marker, name, args_preview, timeout_secs);
    let rgb = tool_rail_rgb(!done, is_error, theme);
    let mut block = Vec::new();
    let mut block_hits = Vec::new();
    push_wrapped(&mut block, &header, inner);
    block_hits.push(CachedFoldHit {
        row_offset: 0,
        col_start: RAILED_MARKER_COL,
        col_end: RAILED_MARKER_COL + mw,
        target: FoldTarget::Tool(id.to_string()),
    });

    if expanded {
        if let Some(content) = write_content
            && !content.is_empty()
        {
            let total = content.lines().count().max(1);
            let opts = ExpandableOutputOptions {
                max_preview_lines: WRITE_BODY_PREVIEW_LINES,
                from: TruncateFrom::Tail,
                expand_hint: format!("{total} total, {CTRL_O_EXPAND_HINT}"),
                hint_style: None,
            };
            push_expandable_with_viewport_hit(
                &mut block,
                &mut block_hits,
                content,
                inner,
                fold.tools_output_expanded,
                &opts,
                RAILED_MARKER_COL,
            );
        }

        if !output.is_empty() {
            let painted = paint_output_with_full_footer(output, theme, is_error);
            let hard = output_is_hard_truncated(output);
            let opts = ExpandableOutputOptions {
                max_preview_lines: TOOLS_OUTPUT_PREVIEW_LINES,
                from: TruncateFrom::Tail,
                expand_hint: if hard {
                    HARD_TRUNCATED_EXPAND_HINT.into()
                } else {
                    CTRL_O_EXPAND_HINT.into()
                },
                hint_style: None,
            };
            let viewport = fold.tools_output_expanded && !hard;
            push_expandable_with_viewport_hit(
                &mut block,
                &mut block_hits,
                &painted,
                inner,
                viewport,
                &opts,
                RAILED_MARKER_COL,
            );
        }

        if let Some(diff) = display_diff
            && !diff.is_empty()
        {
            if !block.is_empty() {
                block.push(String::new());
            }
            push_viewport_diff_lines(
                &mut block,
                &mut block_hits,
                diff,
                inner,
                theme,
                fold.tools_output_expanded,
            );
        }
    }

    let mut lines = Vec::new();
    push_railed(&mut lines, &block, width, rgb);
    (lines, block_hits)
}

/// Diff row: railed summary header + diff viewport (c1350).
pub(super) fn paint_diff_block(
    summary: &str,
    display_diff: &str,
    ctx: PaintCtx,
) -> (Vec<String>, Vec<CachedFoldHit>) {
    let PaintCtx {
        fold,
        glyphs,
        theme,
        width,
    } = ctx;
    let inner = rail_inner_width(width);
    let key = diff_fold_key(summary, display_diff);
    let expanded = fold.tools_effective(&key);
    let marker = if expanded {
        glyphs.unfold()
    } else {
        glyphs.fold()
    };
    let mw = marker_cols(marker);
    let header = paint_tool_header_line(theme, marker, "diff", summary, None);
    let mut block = Vec::new();
    let mut block_hits = Vec::new();
    push_wrapped(&mut block, &header, inner);
    block_hits.push(CachedFoldHit {
        row_offset: 0,
        col_start: RAILED_MARKER_COL,
        col_end: RAILED_MARKER_COL + mw,
        target: FoldTarget::Diff(key),
    });
    let rgb = tool_rail_rgb(false, false, theme);
    if expanded && !display_diff.is_empty() {
        block.push(String::new());
        push_viewport_diff_lines(
            &mut block,
            &mut block_hits,
            display_diff,
            inner,
            theme,
            fold.tools_output_expanded,
        );
    }
    let mut lines = Vec::new();
    push_railed(&mut lines, &block, width, rgb);
    (lines, block_hits)
}

/// Bash row: `$ command` + output viewport or pending hint (c668).
pub(super) fn paint_bash_block(
    command: &str,
    status: BashBlockStatus,
    output: &str,
    ctx: PaintCtx,
) -> (Vec<String>, Vec<CachedFoldHit>) {
    let PaintCtx {
        fold, theme, width, ..
    } = ctx;
    let inner = rail_inner_width(width);
    let mut block = Vec::new();
    let mut block_hits = Vec::new();
    push_wrapped(
        &mut block,
        &theme.paint_success(&format!("$ {command}")),
        inner,
    );
    if !output.is_empty() {
        let body = paint_output_with_full_footer(
            output,
            theme,
            matches!(status, BashBlockStatus::Error | BashBlockStatus::Cancelled),
        );
        let hard = output_is_hard_truncated(output);
        let opts = ExpandableOutputOptions {
            max_preview_lines: TOOLS_OUTPUT_PREVIEW_LINES,
            from: TruncateFrom::Tail,
            expand_hint: if hard {
                HARD_TRUNCATED_EXPAND_HINT.into()
            } else {
                CTRL_O_EXPAND_HINT.into()
            },
            hint_style: None,
        };
        let expanded = fold.tools_output_expanded && !hard;
        push_expandable_with_viewport_hit(
            &mut block,
            &mut block_hits,
            &body,
            inner,
            expanded,
            &opts,
            RAILED_MARKER_COL,
        );
    } else if matches!(status, BashBlockStatus::Pending) {
        push_wrapped(
            &mut block,
            &theme.paint_muted(&format!("Running… {}", key_hint("Esc"))),
            inner,
        );
    }
    let mut lines = Vec::new();
    push_railed(&mut lines, &block, width, bash_rail_rgb(status, theme));
    (lines, block_hits)
}

/// Ask row: accent header + optional detail lines (c1850).
pub(super) fn paint_ask_block(
    id: &str,
    summary: &str,
    detail_lines: &[String],
    phase: AskPhase,
    ctx: PaintCtx,
) -> (Vec<String>, Vec<CachedFoldHit>) {
    let PaintCtx {
        fold,
        glyphs,
        theme,
        width,
    } = ctx;
    let inner = rail_inner_width(width);
    let expanded = fold.tools_effective(id);
    let marker = if expanded {
        glyphs.unfold()
    } else {
        glyphs.fold()
    };
    let mw = marker_cols(marker);
    let header = paint_ask_header_line(theme, marker, summary, inner);
    let mut block = vec![fit(&header, inner)];
    let block_hits = vec![CachedFoldHit {
        row_offset: 0,
        col_start: RAILED_MARKER_COL,
        col_end: RAILED_MARKER_COL + mw,
        target: FoldTarget::Ask(id.to_string()),
    }];
    if expanded {
        for line in detail_lines {
            block.push(fit(&theme.paint_muted(line), inner));
        }
    }
    let mut lines = Vec::new();
    push_railed(&mut lines, &block, width, ask_rail_rgb(phase, theme));
    (lines, block_hits)
}

/// Compaction row: pending hint / foldable summary / failure detail (c1730).
pub(super) fn paint_compaction_block(
    status: CompactionBlockStatus,
    summary: &str,
    tokens_before: u64,
    detail: Option<&str>,
    ctx: PaintCtx,
) -> (Vec<String>, Vec<CachedFoldHit>) {
    let PaintCtx {
        fold,
        glyphs,
        theme,
        width,
    } = ctx;
    let mut lines = Vec::new();
    let mut block_hits = Vec::new();
    match status {
        CompactionBlockStatus::Pending => {
            push_wrapped(
                &mut lines,
                &theme.paint_muted("[compaction] Compacting…"),
                width,
            );
        }
        CompactionBlockStatus::Complete => {
            let n = format_token_count(tokens_before);
            let marker = if fold.compaction_expanded {
                glyphs.unfold()
            } else {
                glyphs.fold()
            };
            let mw = marker_cols(marker);
            let header = if fold.compaction_expanded {
                format!("{marker} [compaction] Compacted from {n} tokens")
            } else {
                format!("{marker} [compaction] Compacted from {n} tokens (Alt+E to expand)")
            };
            let header_row = lines.len();
            push_wrapped(&mut lines, &theme.paint_muted(&header), width);
            block_hits.push(CachedFoldHit {
                row_offset: header_row,
                col_start: 0,
                col_end: mw,
                target: FoldTarget::Compaction,
            });
            if fold.compaction_expanded && !summary.is_empty() {
                lines.push(String::new());
                push_wrapped(&mut lines, &theme.paint_muted(summary), width);
            }
        }
        CompactionBlockStatus::Aborted | CompactionBlockStatus::Failed => {
            let text = detail.unwrap_or("compaction aborted");
            push_wrapped(
                &mut lines,
                &theme.paint_muted(&format!("[compaction] {text}")),
                width,
            );
        }
    }
    (lines, block_hits)
}

/// Todo checklist row: one-line summary + optional items (c1955).
pub(super) fn paint_todo_block(
    summary: &str,
    detail_lines: &[String],
    ctx: PaintCtx,
) -> (Vec<String>, Vec<CachedFoldHit>) {
    let PaintCtx {
        fold,
        glyphs,
        theme,
        width,
    } = ctx;
    let inner = rail_inner_width(width);
    let expanded = fold.todo_expanded;
    let marker = if expanded {
        glyphs.unfold()
    } else {
        glyphs.fold()
    };
    let mw = marker_cols(marker);
    let hint = if expanded {
        String::new()
    } else {
        format!("  {}", key_hint("Alt+E"))
    };
    let header = format!("{marker} {summary}{hint}");
    let mut block = vec![fit(&theme.paint_muted(&header), inner)];
    let block_hits = vec![CachedFoldHit {
        row_offset: 0,
        col_start: RAILED_MARKER_COL,
        col_end: RAILED_MARKER_COL + mw,
        target: FoldTarget::Todo,
    }];
    if expanded {
        for line in detail_lines {
            block.push(fit(&theme.paint_muted(line), inner));
        }
    }
    let rail = {
        let p = theme.palette();
        mix_rgb(p.surface, p.muted, 0.72)
    };
    let mut lines = Vec::new();
    push_railed(&mut lines, &block, width, rail);
    (lines, block_hits)
}

/// Scroll notice row (trailing / navigation instant hints).
pub(super) fn paint_scroll_notice_block(
    text: &str,
    ctx: PaintCtx,
) -> (Vec<String>, Vec<CachedFoldHit>) {
    let PaintCtx {
        glyphs,
        theme,
        width,
        ..
    } = ctx;
    let mut lines = Vec::new();
    push_wrapped(
        &mut lines,
        &theme.paint_muted(&format!("{} {text}", glyphs.system())),
        width,
    );
    (lines, Vec::new())
}

/// Error row.
pub(super) fn paint_error_block(text: &str, ctx: PaintCtx) -> (Vec<String>, Vec<CachedFoldHit>) {
    let PaintCtx { theme, width, .. } = ctx;
    let mut lines = Vec::new();
    push_wrapped(
        &mut lines,
        &theme.paint_error(&format!("error: {text}")),
        width,
    );
    (lines, Vec::new())
}
