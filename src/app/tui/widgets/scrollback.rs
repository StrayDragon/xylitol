//! Live scrollback rendering — Markdown / Expandable / Diff (c476 / c668).
//!
//! Morphology SSOT: `agent_demo` + `design/{markdown,expandable,diff-block,bash-mode}.md`.
//! Not a Codex TranscriptView — lines go into the engine scrollback stack.

use xylitol_tui::{
    Component, DiffInput, DiffOptions, ExpandableOutputOptions, Markdown, TruncateFrom,
    apply_background_to_line, bg_rgb, bold, fg_rgb, render_diff_lines, render_expandable_output,
    truncate_to_width, visible_width, wrap_text_with_ansi,
};

use super::glyphs::GlyphSet;
use crate::app::tui::bridge::{BashBlockStatus, UiEntry, UiModel};
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
}

impl Default for ScrollbackFold {
    fn default() -> Self {
        Self {
            thinking_expanded: false,
            // Product default: tool bodies open; Ctrl+O still clamps viewport height.
            tools_expanded: true,
            tools_output_expanded: false,
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
    block_bg: Option<RgbColor>,
    viewport_full: bool,
) {
    let raw_lines = diff.lines().count();
    let word_level = raw_lines <= WORD_LEVEL_DIFF_LINE_LIMIT;
    let input = DiffInput::DisplayText(diff.to_string());
    let opts = DiffOptions {
        word_level,
        ..DiffOptions::default()
    };
    let diff_theme = match block_bg {
        Some(bg) => theme.palette().diff_theme_on_block(bg),
        None => theme.palette().diff_theme(),
    };
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

/// Full-width tinted row (pad + `apply_background_to_line`).
fn paint_bg_line(line: &str, width: usize, rgb: RgbColor) -> String {
    apply_background_to_line(&fit(line, width), width, &|s| bg_rgb(rgb, s))
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

/// pi `Box` padding_y=1: tinted empty row above/below content; wash spans full terminal width.
fn push_tinted(lines: &mut Vec<String>, content: &[String], width: usize, rgb: RgbColor) {
    lines.push(paint_bg_line("", width, rgb));
    for line in content {
        lines.push(paint_bg_line(line, width, rgb));
    }
    lines.push(paint_bg_line("", width, rgb));
}

fn tool_bg_rgb(pending: bool, is_error: bool, theme: LayoutTheme) -> RgbColor {
    if pending {
        theme.palette().tool_pending_bg
    } else if is_error {
        theme.palette().tool_error_bg
    } else {
        theme.palette().tool_success_bg
    }
}

fn bash_bg_rgb(status: BashBlockStatus, theme: LayoutTheme) -> RgbColor {
    match status {
        BashBlockStatus::Pending => theme.palette().tool_pending_bg,
        BashBlockStatus::Success => theme.palette().tool_success_bg,
        BashBlockStatus::Error | BashBlockStatus::Cancelled => theme.palette().tool_error_bg,
    }
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

/// Per-entry paint cache so streaming/spinner frames do not re-Markdown the
/// entire transcript (ath25).
#[derive(Debug, Default)]
pub struct ScrollbackPaintCache {
    width: usize,
    fold: ScrollbackFold,
    entries: Vec<(u64, Vec<String>)>,
    /// Test/obs: how many committed entries were freshly painted.
    pub(crate) entry_misses: u64,
}

impl ScrollbackPaintCache {
    pub fn invalidate(&mut self) {
        self.entries.clear();
        self.width = 0;
        // keep entry_misses cumulative for tests unless cleared explicitly
    }

    #[cfg(test)]
    #[allow(dead_code)] // called via UiRoot test helper
    pub fn clear_misses(&mut self) {
        self.entry_misses = 0;
    }

    fn prepare(&mut self, width: usize, fold: ScrollbackFold) {
        if self.width != width || self.fold != fold {
            self.entries.clear();
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
        | UiEntry::System { text }
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
                    let content = wrap_text_with_ansi(&body, width);
                    push_tinted(&mut lines, &content, width, theme.palette().user_message_bg);
                }
                UiEntry::Assistant { text } => {
                    let mut md =
                        Markdown::new(text.clone(), 0, 0, theme.palette().markdown_theme(), None);
                    for line in md.render(width) {
                        lines.push(fit(&line, width));
                    }
                }
                UiEntry::Thinking { text } => {
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
                    let marker = if fold.tools_expanded {
                        glyphs.unfold()
                    } else {
                        glyphs.fold()
                    };
                    let header = paint_tool_header_line(theme, marker, name, args_preview);
                    let rgb = tool_bg_rgb(!done, *is_error, theme);
                    let mut block = Vec::new();
                    push_wrapped(&mut block, &header, width);

                    if fold.tools_expanded {
                        // write: header + body share one pending/success/error wash (pi Box).
                        // Tail viewport follows stream end (c1340); Ctrl+O still expands.
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
                                width,
                                fold.tools_output_expanded,
                                &opts,
                            ) {
                                block.push(fit(&line, width));
                            }
                        }

                        // Error / other output behind Alt+E: still same wash when shown.
                        // Hard-truncated: never expand viewport (att16).
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
                            for line in render_expandable_output(&painted, width, expanded, &opts) {
                                block.push(line);
                            }
                        }

                        // edit: header + diff share one tool-*-bg wash; MUST honor Alt+E.
                        if let Some(diff) = display_diff
                            && !diff.is_empty()
                        {
                            if !block.is_empty() {
                                block.push(String::new()); // pi Spacer between title and body
                            }
                            push_viewport_diff_lines(
                                &mut block,
                                diff,
                                width,
                                theme,
                                Some(rgb),
                                fold.tools_output_expanded,
                            );
                        }
                    }

                    push_tinted(&mut lines, &block, width, rgb);
                }
                UiEntry::Diff {
                    summary,
                    display_diff,
                } => {
                    let marker = if fold.tools_expanded {
                        glyphs.unfold()
                    } else {
                        glyphs.fold()
                    };
                    let header = paint_tool_header_line(theme, marker, "diff", summary);
                    let mut block = Vec::new();
                    push_wrapped(&mut block, &header, width);
                    let rgb = tool_bg_rgb(false, false, theme);
                    if fold.tools_expanded && !display_diff.is_empty() {
                        block.push(String::new());
                        push_viewport_diff_lines(
                            &mut block,
                            display_diff,
                            width,
                            theme,
                            Some(rgb),
                            fold.tools_output_expanded,
                        );
                    }
                    push_tinted(&mut lines, &block, width, rgb);
                }
                UiEntry::Bash {
                    command,
                    status,
                    output,
                    ..
                } => {
                    let mut block = Vec::new();
                    push_wrapped(
                        &mut block,
                        &theme.paint_success(&format!("$ {command}")),
                        width,
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
                        for line in render_expandable_output(&body, width, expanded, &opts) {
                            block.push(line);
                        }
                    } else if matches!(status, BashBlockStatus::Pending) {
                        push_wrapped(
                            &mut block,
                            &theme.paint_muted(&format!("Running… {}", key_hint("Esc"))),
                            width,
                        );
                    }
                    push_tinted(&mut lines, &block, width, bash_bg_rgb(*status, theme));
                }
                UiEntry::System { text } => {
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

    for (kind, text) in model.streaming_scrollback_tails() {
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
                let mut md = Markdown::new(
                    format!("{text}…"),
                    0,
                    0,
                    theme.palette().markdown_theme(),
                    None,
                );
                for line in md.render(width) {
                    lines.push(fit(&line, width));
                }
            }
            _ => {}
        }
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tui::bridge::UiEntry;

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

    fn assert_write_header_body_share_bg(done: bool, is_error: bool, expect: RgbColor) {
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
        let bg = format!("\x1b[48;2;{};{};{}m", expect.r, expect.g, expect.b);
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
        assert!(header.contains(&bg), "write header must share wash");
        assert!(
            !header.contains('⚙'),
            "tool header MUST NOT use gear glyph: {header}"
        );
        assert!(
            body.contains(&bg),
            "write body must share the same wash (no naked black split)"
        );
    }

    #[test]
    fn write_block_tints_header_and_body_together() {
        let p = LayoutTheme::product_dark().palette();
        assert_write_header_body_share_bg(false, false, p.tool_pending_bg);
        assert_write_header_body_share_bg(true, false, p.tool_success_bg);
        assert_write_header_body_share_bg(true, true, p.tool_error_bg);
    }

    #[test]
    fn edit_block_tints_header_and_diff_together() {
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
        let success_bg = format!(
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
            header.contains(&success_bg),
            "edit header must use tool-success-bg"
        );
        assert!(
            body.contains(&success_bg),
            "edit diff body must share tool-success-bg wash (no naked black split)"
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
