use xylitol_tui::terminal_colors::RgbColor;
use xylitol_tui::{
    Component, Markdown, bold, fg_rgb, mix_rgb, paint_left_rail_line, truncate_to_width,
    visible_width, wrap_text_with_ansi,
};

use crate::app::tui::bridge::{AskPhase, BashBlockStatus};
use crate::app::tui::layout::LayoutTheme;

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

/// Paint `$name` with `skill_ref` (bold); leave other text unstyled (A10 / c1130).
pub(super) fn highlight_dollar_skill_refs(text: &str, skill_ref: RgbColor) -> String {
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
