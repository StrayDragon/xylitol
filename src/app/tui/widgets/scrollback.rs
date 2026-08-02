//! Live scrollback rendering — Markdown / Expandable / Diff (c476 / c668).
//!
//! Morphology SSOT: `agent_demo` + `design/{markdown,expandable,diff-block,bash-mode}.md`.
//! Not a Codex TranscriptView — lines go into the engine scrollback stack.

use xylitol_tui::{
    Component, DiffInput, DiffOptions, ExpandableOutputOptions, Markdown, TruncateFrom, bold,
    fg_rgb, mix_rgb, paint_left_rail_line, render_diff_lines, render_expandable_output,
    truncate_to_width, visible_width, wrap_text_with_ansi,
};

use super::glyphs::GlyphSet;
use crate::app::tui::bridge::{AskPhase, BashBlockStatus, CompactionBlockStatus, UiEntry, UiModel};
use crate::app::tui::layout::LayoutTheme;
use xylitol_tui::terminal_colors::RgbColor;

/// Fold state owned by the product surface (att7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScrollbackFold {
    pub thinking_expanded: bool,
    /// Alt+E — tool/diff **block** show/hide detail.
    pub tools_expanded: bool,
    /// Ctrl+O — tool/bash detail **viewport** collapsed ↔ full (orthogonal to Alt+E).
    pub tools_output_expanded: bool,
    /// Alt+E — compaction summary (default collapsed; shares chord with tools).
    pub compaction_expanded: bool,
}

impl Default for ScrollbackFold {
    fn default() -> Self {
        Self {
            thinking_expanded: false,
            // Product default: tool bodies open; Ctrl+O still clamps viewport height.
            tools_expanded: true,
            tools_output_expanded: false,
            // Product default: compaction summary collapsed (c1730 / pi).
            compaction_expanded: false,
        }
    }
}

/// Max visual lines for collapsed tool/bash detail (pi bash tool = 5).
const TOOLS_OUTPUT_PREVIEW_LINES: usize = 5;
/// Collapsed write body viewport (pi write.ts = 10 logical lines).
const WRITE_BODY_PREVIEW_LINES: usize = 10;
/// Diff body viewport when Alt+E open but Ctrl+O not yet full.
const DIFF_VIEWPORT_LINES: usize = 12;
/// Max visual lines of edit/Diff body painted into scrollback (c1350).
const MAX_DIFF_RENDER_LINES: usize = 80;
/// Disable word-level when raw display_diff exceeds this many lines.
const WORD_LEVEL_DIFF_LINE_LIMIT: usize = 120;

fn push_viewport_diff_lines(
    lines: &mut Vec<String>,
    diff: &str,
    width: usize,
    theme: LayoutTheme,
    viewport_full: bool,
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
        expand_hint: "ctrl+o to expand".into(),
        hint_style: None,
    };
    for line in render_expandable_output(&body, width, viewport_full, &exp_opts) {
        lines.push(fit(&line, width));
    }
}

fn key_hint(chord: &str) -> String {
    format!("({chord})")
}

/// Ask scrollback header: accent **Ask** + ellipsized ` · q → a · …` rest; fits `inner`.
fn paint_ask_header_line(theme: LayoutTheme, marker: &str, summary: &str, inner: usize) -> String {
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
fn format_token_count(n: u64) -> String {
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

fn paint_tool_header_line(
    theme: LayoutTheme,
    marker: &str,
    name: &str,
    args_preview: &str,
) -> String {
    use crate::app::tui::bridge::display_tool_title;
    let title = display_tool_title(name);
    let hint = theme.paint_muted(&key_hint("Alt+E"));
    if args_preview.is_empty() {
        format!("{marker} {}  {hint}", theme.paint_tool_name(&title))
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
        format!("{marker} {} {}  {hint}", theme.paint_tool_name(&title), loc)
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

fn fit(text: &str, width: usize) -> String {
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

fn push_wrapped(lines: &mut Vec<String>, raw: &str, width: usize) {
    for line in wrap_text_with_ansi(raw, width.max(1)) {
        lines.push(fit(&line, width));
    }
}

/// Untinted full-width row between blocks (pi `Spacer(1)`).
///
/// Must be spaces + `\x1b[49m` — a bare `""` does not reliably occupy a visible
/// terminal row after differential clear, so gaps looked missing.
fn inter_block_spacer(width: usize) -> String {
    format!("{}\x1b[49m", " ".repeat(width.max(1)))
}

/// Content width inside a railed block (rail 1 + gutter 1).
fn rail_inner_width(width: usize) -> usize {
    width.saturating_sub(2).max(1)
}

/// Status rail + gutter for tool/thinking/bash/diff blocks (c1830).
fn push_railed(lines: &mut Vec<String>, content: &[String], width: usize, rgb: RgbColor) {
    for line in content {
        lines.push(paint_left_rail_line(line, width, rgb));
    }
}

fn tool_rail_rgb(pending: bool, is_error: bool, theme: LayoutTheme) -> RgbColor {
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

fn bash_rail_rgb(status: BashBlockStatus, theme: LayoutTheme) -> RgbColor {
    let p = theme.palette();
    let vivid = match status {
        BashBlockStatus::Pending => p.accent,
        BashBlockStatus::Success => p.success,
        BashBlockStatus::Error | BashBlockStatus::Cancelled => p.error,
    };
    mix_rgb(p.surface, vivid, 0.72)
}

fn ask_rail_rgb(phase: AskPhase, theme: LayoutTheme) -> RgbColor {
    let p = theme.palette();
    let vivid = match phase {
        AskPhase::Waiting => p.accent,
        AskPhase::Answered => p.success,
        AskPhase::Skipped => p.muted,
    };
    mix_rgb(p.surface, vivid, 0.72)
}

/// Hard system truncate (c1330/c1340): sidecar Full output footer present.
fn output_is_hard_truncated(output: &str) -> bool {
    output.lines().any(|l| l.starts_with("[Full output:"))
}

const HARD_TRUNCATED_EXPAND_HINT: &str = "expand disabled — see Full output";

/// Paint bash/tool body lines; Full output footer uses warning fg (att15 / pi).
fn paint_output_with_full_footer(output: &str, theme: LayoutTheme, error: bool) -> String {
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

/// Paint `$name` with `skill_ref` (bold); leave other text unstyled (A10 / c1130).
fn highlight_dollar_skill_refs(text: &str, skill_ref: RgbColor) -> String {
    let bytes = text.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' {
            let start = i + 1;
            let mut end = start;
            while end < bytes.len()
                && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_' || bytes[end] == b'-')
            {
                end += 1;
            }
            if end > start {
                let token = &text[i..end];
                out.push_str(&bold(&fg_rgb(skill_ref, token)));
                i = end;
                continue;
            }
        }
        let ch = text[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
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

fn paint_streaming_assistant(
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

/// Per-entry paint cache so streaming/spinner frames do not re-Markdown the
/// entire transcript (ath25).
#[derive(Debug, Default)]
pub struct ScrollbackPaintCache {
    width: usize,
    fold: ScrollbackFold,
    entries: Vec<(u64, Vec<String>)>,
    /// Test/obs: how many committed entries were freshly painted.
    pub(crate) entry_misses: u64,
    /// Streaming assistant incremental paint (ath26).
    pub(crate) streaming_assistant: StreamingAssistantPaint,
}

impl ScrollbackPaintCache {
    pub fn invalidate(&mut self) {
        self.entries.clear();
        self.width = 0;
        self.streaming_assistant.invalidate();
        // keep entry_misses / stream counters cumulative unless cleared
    }

    #[cfg(test)]
    #[allow(dead_code)] // called via UiRoot test helper
    pub fn clear_misses(&mut self) {
        self.entry_misses = 0;
    }

    fn prepare(&mut self, width: usize, fold: ScrollbackFold) {
        if self.width != width || self.fold != fold {
            self.entries.clear();
            self.streaming_assistant.invalidate();
            self.width = width;
            self.fold = fold;
        }
    }
}

fn entry_fingerprint(entry: &UiEntry) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    std::mem::discriminant(entry).hash(&mut h);
    match entry {
        UiEntry::User { text }
        | UiEntry::Assistant { text }
        | UiEntry::Thinking { text }
        | UiEntry::ScrollNotice { text }
        | UiEntry::Error { text } => text.hash(&mut h),
        UiEntry::Tool {
            id,
            name,
            args_preview,
            tool_path,
            write_content,
            display_diff,
            output,
            is_error,
            done,
        } => {
            id.hash(&mut h);
            name.hash(&mut h);
            args_preview.hash(&mut h);
            tool_path.hash(&mut h);
            write_content.hash(&mut h);
            display_diff.hash(&mut h);
            output.hash(&mut h);
            is_error.hash(&mut h);
            done.hash(&mut h);
        }
        UiEntry::Diff {
            summary,
            display_diff,
        } => {
            summary.hash(&mut h);
            display_diff.hash(&mut h);
        }
        UiEntry::Bash {
            command,
            status,
            output,
            exclude_from_context,
        } => {
            command.hash(&mut h);
            status.hash(&mut h);
            output.hash(&mut h);
            exclude_from_context.hash(&mut h);
        }
        UiEntry::Ask {
            id,
            summary,
            detail_lines,
            phase,
            expanded,
        } => {
            id.hash(&mut h);
            summary.hash(&mut h);
            detail_lines.hash(&mut h);
            phase.hash(&mut h);
            expanded.hash(&mut h);
        }
        UiEntry::Compaction {
            status,
            summary,
            tokens_before,
            detail,
        } => {
            status.hash(&mut h);
            summary.hash(&mut h);
            tokens_before.hash(&mut h);
            detail.hash(&mut h);
        }
    }
    h.finish()
}

/// Render UiModel entries into scrollback lines for the product host.
pub fn render_scrollback(
    model: &UiModel,
    glyphs: GlyphSet,
    theme: LayoutTheme,
    fold: ScrollbackFold,
    width: usize,
    cache: &mut ScrollbackPaintCache,
) -> Vec<String> {
    let width = width.max(1);
    cache.prepare(width, fold);
    let mut lines = Vec::new();

    if model.entries.is_empty() && model.streaming_scrollback_tails().is_empty() {
        cache.entries.clear();
        cache.streaming_assistant.invalidate();
        return lines;
    }

    if cache.entries.len() > model.entries.len() {
        cache.entries.truncate(model.entries.len());
    }

    let mut need_spacer = false;
    for (entry_idx, entry) in model.entries.iter().enumerate() {
        if need_spacer {
            lines.push(inter_block_spacer(width));
        }
        need_spacer = true;
        let fp = entry_fingerprint(entry);
        if cache.entries.get(entry_idx).is_some_and(|(f, _)| *f == fp) {
            lines.extend(cache.entries[entry_idx].1.iter().cloned());
            continue;
        }
        let block_lines = {
            let mut lines = Vec::new();
            match entry {
                UiEntry::User { text } => {
                    let prefix = theme.paint_user(glyphs.user());
                    let painted = highlight_dollar_skill_refs(text, theme.palette().skill_ref);
                    let body = format!("{prefix} {painted}");
                    // Flush — no user-message-bg wash, no status rail (atc8 / c1830).
                    push_wrapped(&mut lines, &body, width);
                }
                UiEntry::Assistant { text } => {
                    let mut md =
                        Markdown::new(text.clone(), 0, 0, theme.palette().markdown_theme(), None);
                    for line in md.render(width) {
                        lines.push(fit(&line, width));
                    }
                }
                UiEntry::Thinking { text } => {
                    // Flush like assistant body — thinking is content, not a status tool block.
                    let marker = if fold.thinking_expanded {
                        glyphs.unfold()
                    } else {
                        glyphs.fold()
                    };
                    let header =
                        theme.paint_muted(&format!("{marker} thinking  {}", key_hint("Ctrl+T")));
                    push_wrapped(&mut lines, &header, width);
                    if fold.thinking_expanded {
                        push_wrapped(&mut lines, &theme.paint_muted(text), width);
                    }
                }
                UiEntry::Tool {
                    name,
                    args_preview,
                    write_content,
                    display_diff,
                    output,
                    is_error,
                    done,
                    ..
                } => {
                    let inner = rail_inner_width(width);
                    let marker = if fold.tools_expanded {
                        glyphs.unfold()
                    } else {
                        glyphs.fold()
                    };
                    let header = paint_tool_header_line(theme, marker, name, args_preview);
                    let rgb = tool_rail_rgb(!done, *is_error, theme);
                    let mut block = Vec::new();
                    push_wrapped(&mut block, &header, inner);

                    if fold.tools_expanded {
                        if let Some(content) = write_content
                            && !content.is_empty()
                        {
                            let total = content.lines().count().max(1);
                            let opts = ExpandableOutputOptions {
                                max_preview_lines: WRITE_BODY_PREVIEW_LINES,
                                from: TruncateFrom::Tail,
                                expand_hint: format!("{total} total, ctrl+o to expand"),
                                hint_style: None,
                            };
                            for line in render_expandable_output(
                                content,
                                inner,
                                fold.tools_output_expanded,
                                &opts,
                            ) {
                                block.push(fit(&line, inner));
                            }
                        }

                        if !output.is_empty() {
                            let painted = paint_output_with_full_footer(output, theme, *is_error);
                            let hard = output_is_hard_truncated(output);
                            let opts = ExpandableOutputOptions {
                                max_preview_lines: TOOLS_OUTPUT_PREVIEW_LINES,
                                from: TruncateFrom::Tail,
                                expand_hint: if hard {
                                    HARD_TRUNCATED_EXPAND_HINT.into()
                                } else {
                                    "ctrl+o to expand".into()
                                },
                                hint_style: None,
                            };
                            let expanded = fold.tools_output_expanded && !hard;
                            for line in render_expandable_output(&painted, inner, expanded, &opts) {
                                block.push(fit(&line, inner));
                            }
                        }

                        if let Some(diff) = display_diff
                            && !diff.is_empty()
                        {
                            if !block.is_empty() {
                                block.push(String::new());
                            }
                            push_viewport_diff_lines(
                                &mut block,
                                diff,
                                inner,
                                theme,
                                fold.tools_output_expanded,
                            );
                        }
                    }

                    push_railed(&mut lines, &block, width, rgb);
                }
                UiEntry::Diff {
                    summary,
                    display_diff,
                } => {
                    let inner = rail_inner_width(width);
                    let marker = if fold.tools_expanded {
                        glyphs.unfold()
                    } else {
                        glyphs.fold()
                    };
                    let header = paint_tool_header_line(theme, marker, "diff", summary);
                    let mut block = Vec::new();
                    push_wrapped(&mut block, &header, inner);
                    let rgb = tool_rail_rgb(false, false, theme);
                    if fold.tools_expanded && !display_diff.is_empty() {
                        block.push(String::new());
                        push_viewport_diff_lines(
                            &mut block,
                            display_diff,
                            inner,
                            theme,
                            fold.tools_output_expanded,
                        );
                    }
                    push_railed(&mut lines, &block, width, rgb);
                }
                UiEntry::Bash {
                    command,
                    status,
                    output,
                    ..
                } => {
                    let inner = rail_inner_width(width);
                    let mut block = Vec::new();
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
                                "ctrl+o to expand".into()
                            },
                            hint_style: None,
                        };
                        let expanded = fold.tools_output_expanded && !hard;
                        for line in render_expandable_output(&body, inner, expanded, &opts) {
                            block.push(fit(&line, inner));
                        }
                    } else if matches!(status, BashBlockStatus::Pending) {
                        push_wrapped(
                            &mut block,
                            &theme.paint_muted(&format!("Running… {}", key_hint("Esc"))),
                            inner,
                        );
                    }
                    push_railed(&mut lines, &block, width, bash_rail_rgb(*status, theme));
                }
                UiEntry::Ask {
                    summary,
                    detail_lines,
                    phase,
                    ..
                } => {
                    let inner = rail_inner_width(width);
                    let marker = if fold.tools_expanded {
                        glyphs.unfold()
                    } else {
                        glyphs.fold()
                    };
                    let header = paint_ask_header_line(theme, marker, summary, inner);
                    let mut block = vec![fit(&header, inner)];
                    if fold.tools_expanded {
                        for line in detail_lines {
                            block.push(fit(&theme.paint_muted(line), inner));
                        }
                    }
                    push_railed(&mut lines, &block, width, ask_rail_rgb(*phase, theme));
                }
                UiEntry::Compaction {
                    status,
                    summary,
                    tokens_before,
                    detail,
                } => {
                    push_wrapped(&mut lines, &theme.paint_muted("[compaction]"), width);
                    match status {
                        CompactionBlockStatus::Pending => {
                            push_wrapped(&mut lines, &theme.paint_muted("Compacting…"), width);
                        }
                        CompactionBlockStatus::Complete => {
                            let n = format_token_count(*tokens_before);
                            if fold.compaction_expanded {
                                push_wrapped(
                                    &mut lines,
                                    &theme.paint_muted(&format!("Compacted from {n} tokens")),
                                    width,
                                );
                                if !summary.is_empty() {
                                    lines.push(String::new());
                                    push_wrapped(&mut lines, &theme.paint_muted(summary), width);
                                }
                            } else {
                                push_wrapped(
                                    &mut lines,
                                    &theme.paint_muted(&format!(
                                        "Compacted from {n} tokens (Alt+E to expand)"
                                    )),
                                    width,
                                );
                            }
                        }
                        CompactionBlockStatus::Aborted | CompactionBlockStatus::Failed => {
                            let text = detail.as_deref().unwrap_or("compaction aborted");
                            push_wrapped(&mut lines, &theme.paint_muted(text), width);
                        }
                    }
                }
                UiEntry::ScrollNotice { text } => {
                    push_wrapped(
                        &mut lines,
                        &theme.paint_muted(&format!("{} {text}", glyphs.system())),
                        width,
                    );
                }
                UiEntry::Error { text } => {
                    push_wrapped(
                        &mut lines,
                        &theme.paint_error(&format!("error: {text}")),
                        width,
                    );
                }
            }
            lines // end block paint
        };
        cache.entry_misses = cache.entry_misses.saturating_add(1);
        if entry_idx < cache.entries.len() {
            cache.entries.truncate(entry_idx);
        }
        cache.entries.push((fp, block_lines.clone()));
        lines.extend(block_lines);
    }

    let streaming_tails = model.streaming_scrollback_tails();
    if !streaming_tails.iter().any(|(kind, _)| *kind == "assistant") {
        cache.streaming_assistant.invalidate();
    }

    for (kind, text) in streaming_tails {
        if need_spacer {
            lines.push(inter_block_spacer(width));
        }
        need_spacer = true;
        match kind {
            "thinking" => {
                let marker = if fold.thinking_expanded {
                    glyphs.unfold()
                } else {
                    glyphs.fold()
                };
                let header =
                    theme.paint_muted(&format!("{marker} thinking  {}", key_hint("Ctrl+T")));
                push_wrapped(&mut lines, &header, width);
                if fold.thinking_expanded {
                    push_wrapped(&mut lines, &theme.paint_muted(&format!("{text}…")), width);
                }
            }
            "assistant" => {
                lines.extend(paint_streaming_assistant(
                    text,
                    width,
                    theme,
                    &mut cache.streaming_assistant,
                ));
            }
            _ => {}
        }
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tui::bridge::{AskPhase, CompactionBlockStatus, UiEntry};

    #[test]
    fn ask_header_paints_accent_ask_and_keeps_full_body() {
        let mut model = UiModel::default();
        model.entries.push(UiEntry::Ask {
            id: "a1".into(),
            summary: "Ask · demo_choice → aaa · demo_multi → 📊 看诊断, 📝 查符号".into(),
            detail_lines: vec![
                "demo_choice → aaa".into(),
                "demo_multi → 📊 看诊断, 📝 查符号, 🔍 搜代码, 🚀 跑命令".into(),
            ],
            phase: AskPhase::Answered,
            expanded: true,
        });
        let theme = LayoutTheme::product_dark();
        let fold = ScrollbackFold {
            tools_expanded: true,
            ..ScrollbackFold::default()
        };
        let lines = render_scrollback(
            &model,
            GlyphSet::from_env(),
            theme,
            fold,
            80,
            &mut ScrollbackPaintCache::default(),
        );
        let joined = lines.join("\n");
        let accent_ask = theme.paint_tool_name("Ask");
        assert!(
            joined.contains(&accent_ask),
            "Ask brand must use accent paint; got:\n{joined}"
        );
        let plain = strip_ansi_local(&joined);
        assert!(
            plain.contains("🔍 搜代码") && plain.contains("🚀 跑命令"),
            "expanded body must stay full; got:\n{plain}"
        );
    }

    #[test]
    fn compaction_block_defaults_collapsed() {
        let mut model = UiModel::default();
        model.entries.push(UiEntry::Compaction {
            status: CompactionBlockStatus::Complete,
            summary: "long summary body that should stay hidden".into(),
            tokens_before: 186_842,
            detail: None,
        });
        let theme = LayoutTheme::product_dark();
        let lines = render_scrollback(
            &model,
            GlyphSet::from_env(),
            theme,
            ScrollbackFold::default(),
            100,
            &mut ScrollbackPaintCache::default(),
        );
        let plain = strip_ansi_local(&lines.join("\n"));
        assert!(plain.contains("[compaction]"), "missing label: {plain}");
        assert!(
            plain.contains("Compacted from 186,842 tokens (Alt+E to expand)"),
            "missing collapsed line: {plain}"
        );
        assert!(
            !plain.contains("long summary body"),
            "summary must stay hidden when collapsed: {plain}"
        );
    }

    #[test]
    fn compaction_block_expands_with_fold() {
        let mut model = UiModel::default();
        model.entries.push(UiEntry::Compaction {
            status: CompactionBlockStatus::Complete,
            summary: "visible summary body".into(),
            tokens_before: 1_000,
            detail: None,
        });
        let theme = LayoutTheme::product_dark();
        let lines = render_scrollback(
            &model,
            GlyphSet::from_env(),
            theme,
            ScrollbackFold {
                compaction_expanded: true,
                ..ScrollbackFold::default()
            },
            100,
            &mut ScrollbackPaintCache::default(),
        );
        let plain = strip_ansi_local(&lines.join("\n"));
        assert!(
            plain.contains("Compacted from 1,000 tokens"),
            "missing header: {plain}"
        );
        assert!(
            plain.contains("visible summary body"),
            "expanded must show summary: {plain}"
        );
        assert!(
            !plain.contains("to expand"),
            "expanded must not show expand hint: {plain}"
        );
    }

    #[test]
    fn user_row_highlights_dollar_skill_ref() {
        let mut model = UiModel::default();
        model.entries.push(UiEntry::User {
            text: "please run $demo now".into(),
        });
        let theme = LayoutTheme::product_dark();
        let skill = theme.palette().skill_ref;
        let lines = render_scrollback(
            &model,
            GlyphSet::from_env(),
            theme,
            ScrollbackFold::default(),
            80,
            &mut ScrollbackPaintCache::default(),
        );
        let joined = lines.join("\n");
        let expect = bold(&fg_rgb(skill, "$demo"));
        assert!(
            joined.contains(&expect),
            "user row must paint skill_ref on $demo; got {joined:?}"
        );
    }

    fn assert_write_header_body_share_rail(done: bool, is_error: bool, expect: RgbColor) {
        let mut model = UiModel::default();
        model.entries.push(UiEntry::Tool {
            id: "w1".into(),
            name: "write".into(),
            args_preview: "a.py".into(),
            tool_path: Some("a.py".into()),
            write_content: Some("line-a\nline-b\nline-c\n".into()),
            display_diff: None,
            output: String::new(),
            is_error,
            done,
        });
        let theme = LayoutTheme::product_dark();
        let rail = format!("\x1b[48;2;{};{};{}m", expect.r, expect.g, expect.b);
        let wash = {
            let p = theme.palette();
            let bg = if !done {
                p.tool_pending_bg
            } else if is_error {
                p.tool_error_bg
            } else {
                p.tool_success_bg
            };
            format!("\x1b[48;2;{};{};{}m", bg.r, bg.g, bg.b)
        };
        let lines = render_scrollback(
            &model,
            GlyphSet::from_env(),
            theme,
            ScrollbackFold::default(),
            80,
            &mut ScrollbackPaintCache::default(),
        );
        let header = lines
            .iter()
            .find(|l| l.contains("Write") && l.contains("a.py"))
            .expect("header");
        let body = lines.iter().find(|l| l.contains("line-a")).expect("body");
        assert!(
            header.contains(&rail),
            "write header must share status rail"
        );
        assert!(
            !header.contains('⚙'),
            "tool header MUST NOT use gear glyph: {header}"
        );
        assert!(
            body.contains(&rail),
            "write body must share the same status rail"
        );
        assert!(
            !header.contains(&wash) || wash == rail,
            "write header MUST NOT use full tool-*-bg wash: {header}"
        );
    }

    #[test]
    fn write_block_rails_header_and_body_together() {
        let p = LayoutTheme::product_dark().palette();
        assert_write_header_body_share_rail(false, false, mix_rgb(p.surface, p.accent, 0.72));
        assert_write_header_body_share_rail(true, false, mix_rgb(p.surface, p.success, 0.72));
        assert_write_header_body_share_rail(true, true, mix_rgb(p.surface, p.error, 0.72));
    }

    #[test]
    fn edit_block_rails_header_and_diff_without_wash() {
        let mut model = UiModel::default();
        model.entries.push(UiEntry::Tool {
            id: "e1".into(),
            name: "edit".into(),
            args_preview: "tmp/flow_test.py".into(),
            tool_path: Some("tmp/flow_test.py".into()),
            write_content: None,
            display_diff: Some(
                "@@ -4,3 +4,4 @@\n context-a\n-old line\n+new line\n context-b\n".into(),
            ),
            output: String::new(),
            is_error: false,
            done: true,
        });
        let theme = LayoutTheme::product_dark();
        let p = theme.palette();
        let rail = mix_rgb(p.surface, p.success, 0.72);
        let rail_bg = format!("\x1b[48;2;{};{};{}m", rail.r, rail.g, rail.b);
        let success_wash = format!(
            "\x1b[48;2;{};{};{}m",
            p.tool_success_bg.r, p.tool_success_bg.g, p.tool_success_bg.b
        );
        let added_row_bg = format!(
            "\x1b[48;2;{};{};{}m",
            p.diff_added_bg.r, p.diff_added_bg.g, p.diff_added_bg.b
        );
        let lines = render_scrollback(
            &model,
            GlyphSet::from_env(),
            theme,
            ScrollbackFold::default(),
            100,
            &mut ScrollbackPaintCache::default(),
        );
        let header = lines
            .iter()
            .find(|l| l.contains("Edit") && l.contains("tmp/flow_test.py"))
            .expect("header");
        let body = lines
            .iter()
            .find(|l| l.contains("new line") || l.contains("+new"))
            .expect("diff body");
        assert!(
            header.contains(&rail_bg),
            "edit header must use success rail"
        );
        assert!(
            body.contains(&rail_bg),
            "edit diff body must share success rail (no naked split)"
        );
        assert!(
            !header.contains(&success_wash) || success_wash == rail_bg,
            "edit MUST NOT use tool-success-bg wash envelope"
        );
        assert!(
            !body.contains(&added_row_bg),
            "embedded edit MUST NOT stack diff-added-bg row tint; got {body:?}"
        );
    }

    #[test]
    fn bash_full_output_footer_uses_warning_fg() {
        let mut model = UiModel::default();
        let mut output = String::new();
        for i in 0..20 {
            output.push_str(&format!("line-{i}\n"));
        }
        output.push_str("[Full output: /tmp/x.log. Truncated: 20 lines shown (50.0KB limit)]");
        model.entries.push(UiEntry::Bash {
            command: "big".into(),
            status: BashBlockStatus::Success,
            output,
            exclude_from_context: false,
        });
        let theme = LayoutTheme::product_dark();
        let warning = theme.palette().warning;
        let expect = bold(&fg_rgb(
            warning,
            "[Full output: /tmp/x.log. Truncated: 20 lines shown (50.0KB limit)]",
        ));
        let lines = render_scrollback(
            &model,
            GlyphSet::from_env(),
            theme,
            ScrollbackFold {
                tools_output_expanded: true,
                ..ScrollbackFold::default()
            },
            120,
            &mut ScrollbackPaintCache::default(),
        );
        let joined = lines.join("\n");
        assert!(
            joined.contains(&expect),
            "Full output footer must use warning fg; got {joined:?}"
        );
        let plain = strip_ansi_local(&joined);
        assert!(
            plain.contains("expand disabled"),
            "hard-truncated must not offer ctrl+o expand; got {plain:?}"
        );
        assert!(
            !plain.contains("ctrl+o to expand"),
            "hard-truncated must not show expand hint"
        );
        assert!(
            !plain.contains("line-0"),
            "even with Ctrl+O fold on, hard-truncated must stay on tail; got {plain:?}"
        );
    }

    #[test]
    fn write_viewport_defaults_to_tail_earlier() {
        let mut model = UiModel::default();
        let body: String = (0..18).map(|i| format!("line-{i}\n")).collect();
        model.entries.push(UiEntry::Tool {
            id: "w1".into(),
            name: "write".into(),
            args_preview: "a.py".into(),
            tool_path: Some("a.py".into()),
            write_content: Some(body),
            display_diff: None,
            output: String::new(),
            is_error: false,
            done: false,
        });
        let theme = LayoutTheme::product_dark();
        let lines = render_scrollback(
            &model,
            GlyphSet::from_env(),
            theme,
            ScrollbackFold::default(),
            100,
            &mut ScrollbackPaintCache::default(),
        );
        let plain = strip_ansi_local(&lines.join("\n"));
        assert!(
            plain.contains("earlier lines"),
            "write must use Tail earlier hint; got {plain:?}"
        );
        assert!(
            plain.contains("line-17"),
            "write viewport must show stream end; got {plain:?}"
        );
        let body_at = plain.find("line-17").expect("line-17 present");
        let hint_at = plain.find("earlier lines").expect("earlier hint present");
        assert!(
            hint_at > body_at,
            "earlier hint must be block footer after body; got {plain:?}"
        );
    }

    #[test]
    fn huge_edit_diff_is_capped_in_scrollback() {
        let mut diff = String::new();
        for i in 0..300 {
            diff.push_str(&format!("     {i:>4} | +line-{i}\n"));
        }
        let mut model = UiModel::default();
        model.entries.push(UiEntry::Tool {
            id: "e1".into(),
            name: "edit".into(),
            args_preview: "edit big.txt".into(),
            tool_path: Some("big.txt".into()),
            write_content: None,
            display_diff: Some(diff),
            output: String::new(),
            is_error: false,
            done: true,
        });
        let theme = LayoutTheme::product_dark();
        let lines = render_scrollback(
            &model,
            GlyphSet::from_env(),
            theme,
            ScrollbackFold::default(),
            100,
            &mut ScrollbackPaintCache::default(),
        );
        let plain = strip_ansi_local(&lines.join("\n"));
        assert!(
            plain.contains("omitted") || plain.contains("capped"),
            "huge diff must show cap notice; got {plain:?}"
        );
        assert!(
            lines.len() < 120,
            "scrollback must not paint hundreds of diff rows; got {}",
            lines.len()
        );
    }

    /// c1505 evidence: warm flatten (cache hit + extend) and upper-cache clone
    /// scale with entry count — B-scroll flames miss this because `upper_gen` is
    /// stable while scrolling (no `render_scrollback` re-entry).
    #[test]
    fn scrollback_warm_flatten_grows_with_entry_count() {
        use std::time::Instant;

        fn model_with_n(n: usize) -> UiModel {
            let mut model = UiModel::default();
            for i in 0..n {
                model.entries.push(UiEntry::Assistant {
                    text: format!(
                        "entry-{i}: {}",
                        "这是一段用于 flatten 压力的中文与 `code` 混合正文。".repeat(6)
                    ),
                });
            }
            model
        }

        fn warm_flatten_ns(n: usize, iters: u32) -> (usize, u128) {
            let model = model_with_n(n);
            let theme = LayoutTheme::product_dark();
            let fold = ScrollbackFold::default();
            let glyphs = GlyphSet::from_env();
            let mut cache = ScrollbackPaintCache::default();
            // Cold fill cache.
            let _cold = render_scrollback(&model, glyphs, theme, fold, 100, &mut cache);
            let t0 = Instant::now();
            let mut last_len = 0usize;
            for _ in 0..iters {
                let lines = render_scrollback(&model, glyphs, theme, fold, 100, &mut cache);
                last_len = lines.len();
                // Simulate UiRoot upper_cache hit: clone full upper each frame.
                let _ = lines.clone();
            }
            (last_len, t0.elapsed().as_nanos() / u128::from(iters))
        }

        let iters = 40;
        let (len50, ns50) = warm_flatten_ns(50, iters);
        let (len200, ns200) = warm_flatten_ns(200, iters);
        let (len400, ns400) = warm_flatten_ns(400, iters);
        let report = format!(
            "c1505 flatten evidence: n=50 lines={len50} ~{ns50}ns/iter; \
             n=200 lines={len200} ~{ns200}ns; n=400 lines={len400} ~{ns400}ns\n"
        );
        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/profile/c1505-flatten-microbench.txt");
        if let Some(parent) = out.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&out, &report);
        assert!(
            len200 > len50 * 2,
            "line count should track entries: {report}"
        );
        assert!(len400 > len200, "line count should keep growing: {report}");
        // Allow noise but require clear growth 50 → 400 (warm path).
        assert!(
            ns400 > ns50.saturating_mul(2),
            "warm flatten+clone should grow with history: {report}"
        );
    }

    fn strip_ansi_local(s: &str) -> String {
        let mut out = String::new();
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\u{1b}' {
                if chars.peek() == Some(&'[') {
                    chars.next();
                    for x in chars.by_ref() {
                        if x.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
            } else {
                out.push(c);
            }
        }
        out
    }
}
