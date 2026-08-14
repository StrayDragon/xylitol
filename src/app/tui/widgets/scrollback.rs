//! Live scrollback rendering — Markdown / Expandable / Diff (c476 / c668).
//!
//! Morphology SSOT: `agent_demo` + `design/{markdown,expandable,diff-block,bash-mode}.md`.
//! Not a Codex TranscriptView — lines go into the engine scrollback stack.

use xylitol_tui::{
    Component, DiffInput, DiffOptions, ExpandableOutputOptions, Markdown, TruncateFrom, bold,
    fg_rgb, mix_rgb, paint_left_rail_line, render_diff_lines, render_expandable_output,
    truncate_to_width, visible_width, wrap_text_with_ansi,
};

use std::collections::{HashMap, HashSet};

use super::fold_hit::{FoldHitTable, FoldTarget};
use super::glyphs::GlyphSet;
#[cfg(test)]
use crate::app::tui::activity_fold::is_path_placeholder;
use crate::app::tui::activity_fold::{
    ActivityFoldState, SegmentLevel, cluster_is_thought_only, cluster_middle_indices,
    cluster_omits_header, count_cluster, format_cluster_header, format_elapsed_secs,
    format_envelope_line, middle_entry_indices, partition_segments, streaming_thought_counts,
    thought_header_body,
};
use crate::app::tui::bridge::{
    AskPhase, BashBlockStatus, CompactionBlockStatus, UiEntry, UiModel, UiPhase,
};
use crate::app::tui::layout::LayoutTheme;
use xylitol_tui::terminal_colors::RgbColor;

/// Fold defaults + per-block overrides (att7 / att20 / att21). Not `Copy` — holds maps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrollbackFold {
    pub thinking_expanded: bool,
    /// Alt+E — tool/diff **block** show/hide detail.
    pub tools_expanded: bool,
    /// Ctrl+O — tool/bash detail **viewport** collapsed ↔ full (orthogonal to Alt+E).
    pub tools_output_expanded: bool,
    /// Alt+E — compaction summary (default collapsed; shares chord with tools).
    pub compaction_expanded: bool,
    /// Alt+E — Todo checklist (default collapsed one-line summary; c1955).
    pub todo_expanded: bool,
    /// Per-block tools-family overrides (Tool / Diff / Ask); prefer over [`Self::tools_expanded`].
    pub tools_overrides: HashMap<String, bool>,
    /// Per-id thinking overrides; prefer over [`Self::thinking_expanded`].
    pub thinking_overrides: HashMap<String, bool>,
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
            // Product default: Todo checklist collapsed to summary line (c1955).
            todo_expanded: false,
            tools_overrides: HashMap::new(),
            thinking_overrides: HashMap::new(),
        }
    }
}

/// Defaults that MAY full-clear the paint cache on change.
///
/// `tools_output_expanded` / `compaction_expanded` / `todo_expanded` MUST stay
/// out of this key (ath25 / att29–att30): entry fingerprints already carry them,
/// so Compaction / Ctrl+O / Todo toggles only re-paint affected blocks.
pub type ScrollbackFoldDefaultsKey = (bool, bool);

impl ScrollbackFold {
    pub fn defaults_key(&self) -> ScrollbackFoldDefaultsKey {
        (self.thinking_expanded, self.tools_expanded)
    }

    pub fn tools_effective(&self, id: &str) -> bool {
        self.tools_overrides
            .get(id)
            .copied()
            .unwrap_or(self.tools_expanded)
    }

    pub fn thinking_effective(&self, id: &str) -> bool {
        self.thinking_overrides
            .get(id)
            .copied()
            .unwrap_or(self.thinking_expanded)
    }

    pub fn toggle_tools(&mut self, id: &str) {
        let next = !self.tools_effective(id);
        self.tools_overrides.insert(id.to_string(), next);
    }

    pub fn toggle_thinking(&mut self, id: &str) {
        let next = !self.thinking_effective(id);
        self.thinking_overrides.insert(id.to_string(), next);
    }

    pub fn clear_tools_overrides(&mut self) {
        self.tools_overrides.clear();
    }

    pub fn clear_thinking_overrides(&mut self) {
        self.thinking_overrides.clear();
    }
}

/// Stable Diff fold-key: hash of summary + display_diff.
pub fn diff_fold_key(summary: &str, display_diff: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    summary.hash(&mut h);
    display_diff.hash(&mut h);
    format!("{:016x}", h.finish())
}

/// Triangle column after left rail + gutter (`paint_left_rail_line`).
const RAILED_MARKER_COL: usize = 2;

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

/// Append expandable output; register `OutputViewport` on the visible Ctrl+O hint
/// footer (att30). `col_offset` is screen content col of the inner text (2 when railed).
fn push_expandable_with_viewport_hit(
    lines: &mut Vec<String>,
    block_hits: &mut Vec<CachedFoldHit>,
    text: &str,
    width: usize,
    viewport_full: bool,
    opts: &ExpandableOutputOptions,
    col_offset: usize,
) {
    let out = render_expandable_output(text, width, viewport_full, opts);
    // Hint footer embeds expand_hint inside dim SGR (incl. hard-truncation copy).
    let hint_idx = (!viewport_full)
        .then(|| out.len().checked_sub(1))
        .flatten()
        .filter(|&idx| out[idx].contains(&opts.expand_hint));
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
            target: FoldTarget::OutputViewport,
        });
    }
}

fn push_viewport_diff_lines(
    lines: &mut Vec<String>,
    block_hits: &mut Vec<CachedFoldHit>,
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
    push_expandable_with_viewport_hit(
        lines,
        block_hits,
        &body,
        width,
        viewport_full,
        &exp_opts,
        RAILED_MARKER_COL,
    );
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

/// Relative fold-hit within a cached block (row offset from block start).
#[derive(Debug, Clone)]
struct CachedFoldHit {
    row_offset: usize,
    col_start: usize,
    col_end: usize,
    target: FoldTarget,
}

/// Per-entry paint cache so streaming/spinner frames do not re-Markdown the
/// entire transcript (ath25).
#[derive(Debug, Default)]
pub struct ScrollbackPaintCache {
    width: usize,
    fold_defaults: Option<ScrollbackFoldDefaultsKey>,
    entries: Vec<(u64, Vec<String>, Vec<CachedFoldHit>)>,
    /// Test/obs: how many committed entries were freshly painted.
    pub(crate) entry_misses: u64,
    /// Streaming assistant incremental paint (ath26).
    pub(crate) streaming_assistant: StreamingAssistantPaint,
}

impl ScrollbackPaintCache {
    pub fn invalidate(&mut self) {
        self.entries.clear();
        self.width = 0;
        self.fold_defaults = None;
        self.streaming_assistant.invalidate();
        // keep entry_misses / stream counters cumulative unless cleared
    }

    #[cfg(test)]
    #[allow(dead_code)] // called via UiRoot test helper
    pub fn clear_misses(&mut self) {
        self.entry_misses = 0;
    }

    /// Full-clear only on width / fold **defaults** change — not override-map identity.
    fn prepare(&mut self, width: usize, fold: &ScrollbackFold) {
        let key = fold.defaults_key();
        if self.width != width || self.fold_defaults != Some(key) {
            self.entries.clear();
            self.streaming_assistant.invalidate();
            self.width = width;
            self.fold_defaults = Some(key);
        }
    }
}

fn entry_fingerprint(entry: &UiEntry, fold: &ScrollbackFold) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    std::mem::discriminant(entry).hash(&mut h);
    match entry {
        UiEntry::User { text }
        | UiEntry::Assistant { text }
        | UiEntry::ScrollNotice { text }
        | UiEntry::Error { text } => text.hash(&mut h),
        UiEntry::Thinking {
            id,
            text,
            elapsed_secs,
        } => {
            id.hash(&mut h);
            text.hash(&mut h);
            elapsed_secs.hash(&mut h);
            fold.thinking_effective(id).hash(&mut h);
        }
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
            fold.tools_effective(id).hash(&mut h);
            fold.tools_output_expanded.hash(&mut h);
        }
        UiEntry::Diff {
            summary,
            display_diff,
        } => {
            summary.hash(&mut h);
            display_diff.hash(&mut h);
            let key = diff_fold_key(summary, display_diff);
            fold.tools_effective(&key).hash(&mut h);
            fold.tools_output_expanded.hash(&mut h);
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
            fold.tools_output_expanded.hash(&mut h);
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
            fold.tools_effective(id).hash(&mut h);
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
            fold.compaction_expanded.hash(&mut h);
        }
        UiEntry::Todo {
            summary,
            detail_lines,
        } => {
            summary.hash(&mut h);
            detail_lines.hash(&mut h);
            fold.todo_expanded.hash(&mut h);
        }
    }
    h.finish()
}

fn marker_cols(marker: &str) -> usize {
    visible_width(marker).max(1)
}

fn emit_block_hits(fold_hits: &mut FoldHitTable, content_row_base: usize, hits: &[CachedFoldHit]) {
    for h in hits {
        fold_hits.push(
            content_row_base.saturating_add(h.row_offset),
            h.col_start,
            h.col_end,
            h.target.clone(),
        );
    }
}

/// Render UiModel entries into scrollback lines for the product host.
///
/// Fills `fold_hits.regions` with content-relative rows (caller adds loaded-resources
/// offset). Does not touch `scroll_top` / `transcript_rows`.
///
/// L2/L3 segments skip foldable middles and emit one summary row; segment→row
/// spans land in `activity.row_spans`, and the summary fold marker is also
/// registered on [`FoldHitTable`] as [`FoldTarget::Segment`] (att31 / c2045).
#[allow(clippy::too_many_arguments)] // fold + activity + cache + hits are distinct paint planes
pub fn render_scrollback(
    model: &UiModel,
    glyphs: GlyphSet,
    theme: LayoutTheme,
    fold: &ScrollbackFold,
    activity: &mut ActivityFoldState,
    width: usize,
    cache: &mut ScrollbackPaintCache,
    fold_hits: &mut FoldHitTable,
) -> Vec<String> {
    let width = width.max(1);
    cache.prepare(width, fold);
    fold_hits.clear_regions();
    activity.row_spans.clear();
    let mut lines = Vec::new();

    if model.entries.is_empty() && model.streaming_scrollback_tails().is_empty() {
        cache.entries.clear();
        cache.streaming_assistant.invalidate();
        return lines;
    }

    if cache.entries.len() > model.entries.len() {
        cache.entries.truncate(model.entries.len());
    }

    let segments = partition_segments(&model.entries);
    let has_asst_tail = streaming_assistant_displayable(model);
    let live_seg_idx = live_window_segment_idx(model, &segments, has_asst_tail);
    let mut skip_middle: HashSet<usize> = HashSet::new();
    let mut skip_mid_asst: HashSet<usize> = HashSet::new();
    let mut envelope_summary_at: HashMap<usize, usize> = HashMap::new();
    let mut cluster_header_at: HashMap<usize, (usize, usize)> = HashMap::new();
    let mut thought_only_mids: HashSet<usize> = HashSet::new();
    // Live open cluster only — used by the thinking stream paint. Do not scan
    // older turns (a prior Thought cluster must not swallow the live stream).
    let mut live_open_cluster = false;
    let mut live_open_cluster_expanded = false;
    for (si, seg) in segments.iter().enumerate() {
        let live_seg = live_seg_idx == Some(si);
        let level = activity.effective_level(&seg.id);
        // Envelope header only when the envelope is in play (L2 expanded / L3
        // collapsed). Keep-window L0 and live window stay cluster heads only.
        if activity.settings.paints_envelope_header() && !live_seg && level != SegmentLevel::L0 {
            let mids = middle_entry_indices(&model.entries, seg);
            if let Some(&first) = mids.first() {
                envelope_summary_at.insert(first, si);
            }
        }
        if level == SegmentLevel::L3 {
            let mids = middle_entry_indices(&model.entries, seg);
            skip_middle.extend(mids);
            skip_mid_asst.extend(seg.mid_assistant_idxs.iter().copied());
            continue;
        }
        for (ci, cl) in seg.clusters.iter().enumerate() {
            let mids = cluster_middle_indices(&model.entries, cl);
            let omit_header = cluster_omits_header(&model.entries, cl);
            let sealed_nonempty = mids.iter().any(|&idx| {
                model
                    .entries
                    .get(idx)
                    .is_some_and(|e| !is_inflight_hidden(e) && !is_ask_waiting(e))
            });
            let is_open = is_open_live_cluster(seg, ci, live_seg);
            let has_inflight_tools = mids
                .iter()
                .any(|&idx| model.entries.get(idx).is_some_and(is_inflight_hidden));
            let expanded = activity.cluster_kids_visible(&seg.id, &cl.id);
            // Live open cluster with tools (including inflight) gets a foldable
            // cluster header. Kids stay collapsed until the user opens them —
            // paint must not auto-expand (avoids popping bodies and per-frame HashSet writes).
            let paint_header =
                !omit_header && !(is_open && !sealed_nonempty && !has_inflight_tools);
            if paint_header && let Some(&first) = mids.first() {
                cluster_header_at.insert(first, (si, ci));
            }
            let thought_only = cluster_is_thought_only(&model.entries, cl);
            if thought_only {
                thought_only_mids.extend(mids.iter().copied());
            }
            if is_open {
                live_open_cluster = true;
                live_open_cluster_expanded = expanded;
            }
            for &idx in &mids {
                let Some(entry) = model.entries.get(idx) else {
                    continue;
                };
                if is_ask_waiting(entry) {
                    continue;
                }
                if omit_header {
                    // Envelope is already expanded here; show the compaction block.
                    continue;
                }
                if !expanded {
                    skip_middle.insert(idx);
                }
            }
        }
    }

    let mut need_spacer = false;
    for (entry_idx, entry) in model.entries.iter().enumerate() {
        if skip_mid_asst.contains(&entry_idx) {
            continue;
        }
        if let Some(&si) = envelope_summary_at.get(&entry_idx) {
            paint_envelope_header_row(
                &mut lines,
                fold_hits,
                activity,
                &segments[si],
                glyphs,
                theme,
                width,
                &mut need_spacer,
            );
            if activity.effective_level(&segments[si].id) == SegmentLevel::L3 {
                continue;
            }
        }
        if skip_middle.contains(&entry_idx) {
            if let Some(&(si, ci)) = cluster_header_at.get(&entry_idx) {
                paint_cluster_header_row(
                    &mut lines,
                    fold_hits,
                    &segments,
                    si,
                    ci,
                    model,
                    activity,
                    live_seg_idx,
                    glyphs,
                    theme,
                    width,
                    &mut need_spacer,
                );
            }
            continue;
        }
        if let Some(&(si, ci)) = cluster_header_at.get(&entry_idx) {
            paint_cluster_header_row(
                &mut lines,
                fold_hits,
                &segments,
                si,
                ci,
                model,
                activity,
                live_seg_idx,
                glyphs,
                theme,
                width,
                &mut need_spacer,
            );
        }
        let glued_to_header = cluster_header_at.contains_key(&entry_idx);
        if need_spacer && !glued_to_header {
            lines.push(inter_block_spacer(width));
        }
        need_spacer = true;
        let fp = entry_fingerprint(entry, fold);
        if cache
            .entries
            .get(entry_idx)
            .is_some_and(|(f, _, _)| *f == fp)
        {
            let content_row_base = lines.len();
            let (_, block_lines, hits) = &cache.entries[entry_idx];
            emit_block_hits(fold_hits, content_row_base, hits);
            lines.extend(block_lines.iter().cloned());
            continue;
        }
        let mut block_hits = Vec::new();
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
                UiEntry::Thinking {
                    id,
                    text,
                    elapsed_secs,
                } => {
                    if thought_only_mids.contains(&entry_idx) {
                        // Cluster header is already Thought; don't paint a second L1 row.
                        push_wrapped(&mut lines, &theme.paint_muted(text), width);
                    } else {
                        let expanded = fold.thinking_effective(id);
                        let marker = if expanded {
                            glyphs.unfold()
                        } else {
                            glyphs.fold()
                        };
                        let mw = marker_cols(marker);
                        let dur = elapsed_secs.filter(|s| *s > 0).map(format_elapsed_secs);
                        let label = thought_header_body(dur.as_deref());
                        let header =
                            theme.paint_muted(&format!("{marker} {label}  {}", key_hint("Ctrl+T")));
                        let header_row = lines.len();
                        push_wrapped(&mut lines, &header, width);
                        block_hits.push(CachedFoldHit {
                            row_offset: header_row,
                            col_start: 0,
                            col_end: mw,
                            target: FoldTarget::Thinking(id.clone()),
                        });
                        if expanded {
                            push_wrapped(&mut lines, &theme.paint_muted(text), width);
                        }
                    }
                }
                UiEntry::Tool {
                    id,
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
                    let expanded = fold.tools_effective(id);
                    let marker = if expanded {
                        glyphs.unfold()
                    } else {
                        glyphs.fold()
                    };
                    let mw = marker_cols(marker);
                    let header = paint_tool_header_line(theme, marker, name, args_preview);
                    let rgb = tool_rail_rgb(!done, *is_error, theme);
                    let mut block = Vec::new();
                    push_wrapped(&mut block, &header, inner);
                    block_hits.push(CachedFoldHit {
                        row_offset: 0,
                        col_start: RAILED_MARKER_COL,
                        col_end: RAILED_MARKER_COL + mw,
                        target: FoldTarget::Tool(id.clone()),
                    });

                    if expanded {
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

                    push_railed(&mut lines, &block, width, rgb);
                }
                UiEntry::Diff {
                    summary,
                    display_diff,
                } => {
                    let inner = rail_inner_width(width);
                    let key = diff_fold_key(summary, display_diff);
                    let expanded = fold.tools_effective(&key);
                    let marker = if expanded {
                        glyphs.unfold()
                    } else {
                        glyphs.fold()
                    };
                    let mw = marker_cols(marker);
                    let header = paint_tool_header_line(theme, marker, "diff", summary);
                    let mut block = Vec::new();
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
                    push_railed(&mut lines, &block, width, bash_rail_rgb(*status, theme));
                }
                UiEntry::Ask {
                    id,
                    summary,
                    detail_lines,
                    phase,
                    ..
                } => {
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
                    block_hits.push(CachedFoldHit {
                        row_offset: 0,
                        col_start: RAILED_MARKER_COL,
                        col_end: RAILED_MARKER_COL + mw,
                        target: FoldTarget::Ask(id.clone()),
                    });
                    if expanded {
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
                } => match status {
                    CompactionBlockStatus::Pending => {
                        push_wrapped(
                            &mut lines,
                            &theme.paint_muted("[compaction] Compacting…"),
                            width,
                        );
                    }
                    CompactionBlockStatus::Complete => {
                        let n = format_token_count(*tokens_before);
                        let marker = if fold.compaction_expanded {
                            glyphs.unfold()
                        } else {
                            glyphs.fold()
                        };
                        let mw = marker_cols(marker);
                        let header = if fold.compaction_expanded {
                            format!("{marker} [compaction] Compacted from {n} tokens")
                        } else {
                            format!(
                                "{marker} [compaction] Compacted from {n} tokens (Alt+E to expand)"
                            )
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
                        let text = detail.as_deref().unwrap_or("compaction aborted");
                        push_wrapped(
                            &mut lines,
                            &theme.paint_muted(&format!("[compaction] {text}")),
                            width,
                        );
                    }
                },
                UiEntry::Todo {
                    summary,
                    detail_lines,
                } => {
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
                    block_hits.push(CachedFoldHit {
                        row_offset: 0,
                        col_start: RAILED_MARKER_COL,
                        col_end: RAILED_MARKER_COL + mw,
                        target: FoldTarget::Todo,
                    });
                    if expanded {
                        for line in detail_lines {
                            block.push(fit(&theme.paint_muted(line), inner));
                        }
                    }
                    let rail = {
                        let p = theme.palette();
                        mix_rgb(p.surface, p.muted, 0.72)
                    };
                    push_railed(&mut lines, &block, width, rail);
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
        let content_row_base = lines.len();
        emit_block_hits(fold_hits, content_row_base, &block_hits);
        cache.entries.push((fp, block_lines.clone(), block_hits));
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
                if activity.settings.enabled {
                    paint_folded_streaming_thought(
                        &mut lines,
                        fold_hits,
                        activity,
                        model,
                        &segments,
                        live_open_cluster,
                        live_open_cluster_expanded,
                        text,
                        glyphs,
                        theme,
                        width,
                    );
                } else {
                    // att8 / att21: without ActivityFold, streaming thinking stays expanded.
                    let marker = glyphs.unfold();
                    let header =
                        theme.paint_muted(&format!("{marker} Thinking  {}", key_hint("Ctrl+T")));
                    push_wrapped(&mut lines, &header, width);
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

    if activity.settings.enabled
        && model.phase == UiPhase::Busy
        && let Some(label) = live_tail_label(model, &segments, has_asst_tail)
    {
        if need_spacer {
            lines.push(inter_block_spacer(width));
        }
        let painted = theme.paint_muted(&label);
        let row_start = lines.len();
        push_wrapped(&mut lines, &painted, width);
        fold_hits.push(row_start, 0, width.max(1), FoldTarget::LiveTail);
    }

    lines
}

#[allow(clippy::too_many_arguments)] // paint planes: lines, hits, activity, theme
fn paint_envelope_header_row(
    lines: &mut Vec<String>,
    fold_hits: &mut FoldHitTable,
    activity: &mut ActivityFoldState,
    seg: &crate::app::tui::activity_fold::ActivitySegment,
    glyphs: GlyphSet,
    theme: LayoutTheme,
    width: usize,
    need_spacer: &mut bool,
) {
    if *need_spacer {
        lines.push(inter_block_spacer(width));
    }
    let expanded = activity.effective_level(&seg.id) != SegmentLevel::L3;
    let dur = activity.duration_for(&seg.id);
    let plain = format_envelope_line(glyphs, dur.as_deref(), expanded);
    let marker = if expanded {
        glyphs.unfold()
    } else {
        glyphs.fold()
    };
    let mw = marker_cols(marker);
    let painted = theme.paint_muted(&plain);
    let row_start = lines.len();
    push_wrapped(lines, &painted, width);
    let row_end = lines.len();
    fold_hits.push(row_start, 0, mw, FoldTarget::Segment(seg.id.clone()));
    activity
        .row_spans
        .insert(seg.id.clone(), row_start, row_end);
    *need_spacer = true;
}

#[allow(clippy::too_many_arguments)] // paint planes: lines, hits, segments, activity, theme
fn paint_cluster_header_row(
    lines: &mut Vec<String>,
    fold_hits: &mut FoldHitTable,
    segments: &[crate::app::tui::activity_fold::ActivitySegment],
    si: usize,
    ci: usize,
    model: &UiModel,
    activity: &ActivityFoldState,
    live_seg_idx: Option<usize>,
    glyphs: GlyphSet,
    theme: LayoutTheme,
    width: usize,
    need_spacer: &mut bool,
) {
    let seg = &segments[si];
    let cl = &seg.clusters[ci];
    if *need_spacer {
        lines.push(inter_block_spacer(width));
    }
    let live_seg = live_seg_idx == Some(si);
    let expanded = activity.cluster_kids_visible(&seg.id, &cl.id);
    let progressive = is_open_live_cluster(seg, ci, live_seg);
    let counts = count_cluster(&model.entries, cl);
    let thought_dur = counts
        .is_thought_only()
        .then(|| thought_duration_label(model, cl))
        .flatten();
    let live_thinking =
        progressive && counts.is_thought_only() && !model.streaming_thinking.is_empty();
    let plain = format_cluster_header(
        glyphs,
        &counts,
        expanded,
        progressive,
        thought_dur.as_deref(),
        live_thinking,
    );
    let marker = if expanded {
        glyphs.unfold()
    } else {
        glyphs.fold()
    };
    let mw = marker_cols(marker);
    let painted = theme.paint_muted(&plain);
    let row_start = lines.len();
    push_wrapped(lines, &painted, width);
    fold_hits.push(row_start, 0, mw, FoldTarget::Cluster(cl.id.clone()));
    *need_spacer = true;
}

/// ActivityFold on: merge streaming Think into a Thinking bar (no second L1 header).
///
/// Cluster id matches the cluster `partition_segments` will assign when the
/// stream flushes, so a user expand survives ToolStart / MessageEnd.
#[allow(clippy::too_many_arguments)] // paint planes: lines, hits, activity, theme
fn paint_folded_streaming_thought(
    lines: &mut Vec<String>,
    fold_hits: &mut FoldHitTable,
    activity: &ActivityFoldState,
    model: &UiModel,
    segments: &[crate::app::tui::activity_fold::ActivitySegment],
    live_open_cluster: bool,
    live_open_cluster_expanded: bool,
    text: &str,
    glyphs: GlyphSet,
    theme: LayoutTheme,
    width: usize,
) {
    if live_open_cluster {
        // Header already painted (Thinking/Thought or Editing/Exploring/…).
        // Thinking stream is a kid — no second L1 row.
        if live_open_cluster_expanded {
            push_wrapped(lines, &theme.paint_muted(&format!("{text}…")), width);
        }
        return;
    }

    let (env_id, cluster_id) = next_live_thought_cluster_id(&model.entries, segments);
    let expanded = activity.cluster_kids_visible(&env_id, &cluster_id);
    let counts = streaming_thought_counts();
    let plain = format_cluster_header(glyphs, &counts, expanded, true, None, true);
    let marker = if expanded {
        glyphs.unfold()
    } else {
        glyphs.fold()
    };
    let mw = marker_cols(marker);
    let painted = theme.paint_muted(&plain);
    let row_start = lines.len();
    push_wrapped(lines, &painted, width);
    fold_hits.push(row_start, 0, mw, FoldTarget::Cluster(cluster_id));
    if expanded {
        push_wrapped(lines, &theme.paint_muted(&format!("{text}…")), width);
    }
}

fn thought_duration_label(
    model: &UiModel,
    cluster: &crate::app::tui::activity_fold::ActivityCluster,
) -> Option<String> {
    let mut secs = 0u64;
    for idx in cluster_middle_indices(&model.entries, cluster) {
        if let Some(UiEntry::Thinking {
            elapsed_secs: Some(s),
            ..
        }) = model.entries.get(idx)
        {
            secs = secs.saturating_add(*s);
        }
    }
    (secs > 0).then(|| format_elapsed_secs(secs))
}

fn next_live_thought_cluster_id(
    entries: &[UiEntry],
    segments: &[crate::app::tui::activity_fold::ActivitySegment],
) -> (String, String) {
    if let Some(seg) = segments.last() {
        let id = format!("{}:c{}", seg.id, seg.clusters.len());
        return (seg.id.clone(), id);
    }
    let user_idx = entries
        .iter()
        .rposition(|e| matches!(e, UiEntry::User { .. }))
        .unwrap_or(0);
    let env = format!("seg-{user_idx}");
    (env.clone(), format!("{env}:c0"))
}

fn is_open_live_cluster(
    seg: &crate::app::tui::activity_fold::ActivitySegment,
    ci: usize,
    live_seg: bool,
) -> bool {
    live_seg && ci + 1 == seg.clusters.len() && seg.clusters[ci].seal_assistant_idx.is_none()
}

fn streaming_assistant_displayable(model: &UiModel) -> bool {
    model
        .streaming_scrollback_tails()
        .iter()
        .any(|(kind, text)| *kind == "assistant" && text.chars().any(|c| !c.is_whitespace()))
}

fn live_window_segment_idx(
    model: &UiModel,
    segments: &[crate::app::tui::activity_fold::ActivitySegment],
    has_asst_tail: bool,
) -> Option<usize> {
    if model.phase != UiPhase::Busy || has_asst_tail {
        return None;
    }
    let (si, seg) = segments.iter().enumerate().next_back()?;
    let last = seg.clusters.last()?;
    if last.seal_assistant_idx.is_some() {
        return None;
    }
    Some(si)
}

fn is_ask_waiting(entry: &UiEntry) -> bool {
    matches!(
        entry,
        UiEntry::Ask {
            phase: AskPhase::Waiting,
            ..
        }
    )
}

fn is_inflight_hidden(entry: &UiEntry) -> bool {
    matches!(
        entry,
        UiEntry::Tool { done: false, .. }
            | UiEntry::Bash {
                status: BashBlockStatus::Pending,
                ..
            }
    )
}

#[cfg(test)]
fn inflight_short_label(entry: &UiEntry) -> Option<String> {
    match entry {
        UiEntry::Tool {
            done: false,
            name,
            args_preview,
            tool_path,
            ..
        } => {
            let file = tool_path
                .as_deref()
                .filter(|s| !is_path_placeholder(s))
                .unwrap_or(args_preview.as_str());
            let file = if is_path_placeholder(file) {
                None
            } else {
                Some(file)
            };
            let n = name.to_ascii_lowercase();
            let label = if matches!(
                n.as_str(),
                "edit" | "write" | "apply_patch" | "strreplace" | "str_replace"
            ) {
                match file {
                    Some(file) => format!("Editing {file}"),
                    None => "Editing".into(),
                }
            } else if n == "read" || n == "cat" {
                match file {
                    Some(file) => format!("Reading {file}"),
                    None => "Reading".into(),
                }
            } else if is_searchish(&n) {
                match file {
                    Some(file) => format!("Searching {file}"),
                    None => "Searching".into(),
                }
            } else if matches!(
                n.as_str(),
                "bash" | "shell" | "run_terminal_cmd" | "execute"
            ) {
                match file {
                    Some(file) => format!("Running {file}"),
                    None => "Running".into(),
                }
            } else {
                format!("Running {name}")
            };
            Some(label)
        }
        UiEntry::Bash {
            status: BashBlockStatus::Pending,
            command,
            ..
        } => Some(format!("Running {command}")),
        _ => None,
    }
}

#[cfg(test)]
fn is_searchish(name: &str) -> bool {
    matches!(
        name,
        "grep" | "rg" | "search" | "glob" | "find" | "codebase_search" | "semantic_search"
    ) || name.contains("search")
        || name.contains("grep")
}

fn live_tail_label(
    model: &UiModel,
    segments: &[crate::app::tui::activity_fold::ActivitySegment],
    has_asst_tail: bool,
) -> Option<String> {
    if has_asst_tail {
        return None;
    }
    let last_cluster_unsealed = segments.last().is_none_or(|s| {
        s.clusters
            .last()
            .is_none_or(|c| c.seal_assistant_idx.is_none())
    });
    if !last_cluster_unsealed {
        return None;
    }
    if model.entries.iter().rev().any(is_ask_waiting) {
        return Some("Asking questions".into());
    }
    None
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
            &fold,
            &mut crate::app::tui::activity_fold::ActivityFoldState::default(),
            80,
            &mut ScrollbackPaintCache::default(),
            &mut FoldHitTable::default(),
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
    fn todo_checklist_defaults_to_summary_line() {
        let mut model = UiModel::default();
        model.entries.push(UiEntry::Todo {
            summary: "Todo · 2/5".into(),
            detail_lines: vec!["[x] done".into(), "[ ] next".into()],
        });
        let theme = LayoutTheme::product_dark();
        let lines = render_scrollback(
            &model,
            GlyphSet::from_env(),
            theme,
            &ScrollbackFold::default(),
            &mut crate::app::tui::activity_fold::ActivityFoldState::default(),
            80,
            &mut ScrollbackPaintCache::default(),
            &mut FoldHitTable::default(),
        );
        let plain = strip_ansi_local(&lines.join("\n"));
        assert!(plain.contains("Todo · 2/5"), "missing summary: {plain}");
        assert!(
            plain.contains("(Alt+E)") && !plain.contains("((Alt+E))"),
            "folded hint MUST be a single pair of parens: {plain}"
        );
        assert!(
            !plain.contains("[x] done"),
            "detail must stay folded: {plain}"
        );
        assert!(
            !plain.to_lowercase().contains("plan"),
            "must not render Plan side chrome: {plain}"
        );
    }

    #[test]
    fn todo_checklist_expands_with_fold() {
        let mut model = UiModel::default();
        model.entries.push(UiEntry::Todo {
            summary: "Todo · 1/2".into(),
            detail_lines: vec!["[x] done".into(), "[~] wip".into()],
        });
        let theme = LayoutTheme::product_dark();
        let lines = render_scrollback(
            &model,
            GlyphSet::from_env(),
            theme,
            &ScrollbackFold {
                todo_expanded: true,
                ..ScrollbackFold::default()
            },
            &mut crate::app::tui::activity_fold::ActivityFoldState::default(),
            80,
            &mut ScrollbackPaintCache::default(),
            &mut FoldHitTable::default(),
        );
        let plain = strip_ansi_local(&lines.join("\n"));
        assert!(
            plain.contains("[x] done") && plain.contains("[~] wip"),
            "{plain}"
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
            &ScrollbackFold::default(),
            &mut crate::app::tui::activity_fold::ActivityFoldState::default(),
            100,
            &mut ScrollbackPaintCache::default(),
            &mut FoldHitTable::default(),
        );
        let plain = strip_ansi_local(&lines.join("\n"));
        let header = plain
            .lines()
            .find(|l| l.contains("Compacted from"))
            .unwrap_or("");
        assert!(
            header.contains("[compaction]")
                && header.contains("Compacted from 186,842 tokens (Alt+E to expand)")
                && (header.contains('▸') || header.contains('>')),
            "fold triangle MUST sit on the Compacted header line: {plain}"
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
            &ScrollbackFold {
                compaction_expanded: true,
                ..ScrollbackFold::default()
            },
            &mut crate::app::tui::activity_fold::ActivityFoldState::default(),
            100,
            &mut ScrollbackPaintCache::default(),
            &mut FoldHitTable::default(),
        );
        let plain = strip_ansi_local(&lines.join("\n"));
        let header = plain
            .lines()
            .find(|l| l.contains("Compacted from"))
            .unwrap_or("");
        assert!(
            header.contains("[compaction]")
                && header.contains("Compacted from 1,000 tokens")
                && (header.contains('▾') || header.contains('v')),
            "expanded triangle MUST sit on the Compacted header line: {plain}"
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
    fn inflight_write_placeholder_is_editing_not_dots() {
        let entry = UiEntry::Tool {
            id: "w1".into(),
            name: "write".into(),
            args_preview: "...".into(),
            tool_path: None,
            write_content: None,
            display_diff: None,
            output: String::new(),
            is_error: false,
            done: false,
        };
        assert_eq!(inflight_short_label(&entry).as_deref(), Some("Editing"));
        let with_path = UiEntry::Tool {
            id: "w2".into(),
            name: "write".into(),
            args_preview: "a.py".into(),
            tool_path: Some("a.py".into()),
            write_content: None,
            display_diff: None,
            output: String::new(),
            is_error: false,
            done: false,
        };
        assert_eq!(
            inflight_short_label(&with_path).as_deref(),
            Some("Editing a.py")
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
            &ScrollbackFold::default(),
            &mut crate::app::tui::activity_fold::ActivityFoldState::default(),
            80,
            &mut ScrollbackPaintCache::default(),
            &mut FoldHitTable::default(),
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
            &ScrollbackFold::default(),
            &mut crate::app::tui::activity_fold::ActivityFoldState::default(),
            80,
            &mut ScrollbackPaintCache::default(),
            &mut FoldHitTable::default(),
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
            &ScrollbackFold::default(),
            &mut crate::app::tui::activity_fold::ActivityFoldState::default(),
            100,
            &mut ScrollbackPaintCache::default(),
            &mut FoldHitTable::default(),
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
            &ScrollbackFold {
                tools_output_expanded: true,
                ..ScrollbackFold::default()
            },
            &mut crate::app::tui::activity_fold::ActivityFoldState::default(),
            120,
            &mut ScrollbackPaintCache::default(),
            &mut FoldHitTable::default(),
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
            &ScrollbackFold::default(),
            &mut crate::app::tui::activity_fold::ActivityFoldState::default(),
            100,
            &mut ScrollbackPaintCache::default(),
            &mut FoldHitTable::default(),
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
            &ScrollbackFold::default(),
            &mut crate::app::tui::activity_fold::ActivityFoldState::default(),
            100,
            &mut ScrollbackPaintCache::default(),
            &mut FoldHitTable::default(),
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

    #[test]
    fn live_window_omits_planning_placeholder() {
        let mut model = UiModel::default();
        model.phase = UiPhase::Busy;
        model.entries.push(UiEntry::User { text: "go".into() });
        let mut hits = FoldHitTable::default();
        let lines = render_scrollback(
            &model,
            GlyphSet::from_env(),
            LayoutTheme::product_dark(),
            &ScrollbackFold::default(),
            &mut crate::app::tui::activity_fold::ActivityFoldState::default(),
            80,
            &mut ScrollbackPaintCache::default(),
            &mut hits,
        );
        let plain = strip_ansi_local(&lines.join("\n"));
        assert!(
            !plain.contains("Planning next moves"),
            "busy chrome is status spinner, not a Planning placeholder: {plain}"
        );
        assert!(
            !plain.contains("Worked for"),
            "live window must not wrap current turn in Worked for: {plain}"
        );
        assert!(
            !hits
                .regions
                .iter()
                .any(|r| matches!(r.target, FoldTarget::LiveTail)),
            "no Planning live-tail hit: {:?}",
            hits.regions
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
            let _cold = render_scrollback(
                &model,
                glyphs,
                theme,
                &fold,
                &mut crate::app::tui::activity_fold::ActivityFoldState::default(),
                100,
                &mut cache,
                &mut FoldHitTable::default(),
            );
            let t0 = Instant::now();
            let mut last_len = 0usize;
            for _ in 0..iters {
                let lines = render_scrollback(
                    &model,
                    glyphs,
                    theme,
                    &fold,
                    &mut crate::app::tui::activity_fold::ActivityFoldState::default(),
                    100,
                    &mut cache,
                    &mut FoldHitTable::default(),
                );
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
