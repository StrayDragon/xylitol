//! Live scrollback rendering — Markdown / Expandable / Diff (c476).
//!
//! Morphology SSOT: `agent_demo` + `design/{markdown,expandable,diff-block}.md`.
//! Not a Codex TranscriptView — lines go into the engine scrollback stack.

use xylitol_tui::{
    Component, DiffInput, DiffOptions, ExpandableOutputOptions, Markdown, TruncateFrom,
    apply_background_to_line, bg_rgb, render_diff_lines, render_expandable_output,
    truncate_to_width, visible_width, wrap_text_with_ansi,
};

use super::glyphs::GlyphSet;
use crate::app::tui::bridge::{UiEntry, UiModel};
use crate::app::tui::layout::LayoutTheme;

/// Fold state owned by the product surface (att7).
#[derive(Debug, Clone, Copy, Default)]
pub struct ScrollbackFold {
    pub thinking_expanded: bool,
    pub tools_expanded: bool,
}

fn key_hint(chord: &str) -> String {
    format!("({chord})")
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

fn paint_tool_bg(
    line: &str,
    width: usize,
    pending: bool,
    is_error: bool,
    theme: LayoutTheme,
) -> String {
    let rgb = if pending {
        theme.palette().tool_pending_bg
    } else if is_error {
        theme.palette().tool_error_bg
    } else {
        theme.palette().tool_success_bg
    };
    apply_background_to_line(line, width, &|s| bg_rgb(rgb, s))
}

/// Render UiModel entries into scrollback lines for the product host.
pub fn render_scrollback(
    model: &UiModel,
    glyphs: GlyphSet,
    theme: LayoutTheme,
    fold: ScrollbackFold,
    width: usize,
) -> Vec<String> {
    let width = width.max(1);
    let mut lines = Vec::new();

    if model.entries.is_empty() && model.streaming_scrollback_tails().is_empty() {
        return lines;
    }

    for entry in &model.entries {
        match entry {
            UiEntry::User { text } => {
                let prefix = theme.paint_user(glyphs.user());
                let body = format!("{prefix} {text}");
                // Optional user-message-bg on each wrapped line.
                let bg = theme.palette().user_message_bg;
                for line in wrap_text_with_ansi(&body, width) {
                    let fitted = fit(&line, width);
                    lines.push(apply_background_to_line(&fitted, width, &|s| bg_rgb(bg, s)));
                }
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
                let state = if !done {
                    "…"
                } else if *is_error {
                    "err"
                } else {
                    "ok"
                };
                let summary = if args_preview.is_empty() {
                    format!("{name} [{state}]")
                } else {
                    format!("{name} [{state}] {args_preview}")
                };
                let header = format!(
                    "{marker} {} {summary}  {}",
                    glyphs.tool(),
                    key_hint("Alt+E")
                );
                let mut block = Vec::new();
                push_wrapped(&mut block, &theme.paint_tool(&header), width);
                if fold.tools_expanded && !output.is_empty() {
                    let opts = ExpandableOutputOptions {
                        max_preview_lines: 12,
                        from: TruncateFrom::Tail,
                        expand_hint: "ctrl+o to expand".into(),
                        hint_style: None,
                    };
                    for line in render_expandable_output(output, width, true, &opts) {
                        block.push(fit(&line, width));
                    }
                }
                for line in block {
                    lines.push(paint_tool_bg(&line, width, !done, *is_error, theme));
                }
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
                let header = format!(
                    "{marker} {} {summary}  {}",
                    glyphs.tool(),
                    key_hint("Alt+E")
                );
                let mut header_lines = Vec::new();
                push_wrapped(&mut header_lines, &theme.paint_tool(&header), width);
                for line in header_lines {
                    lines.push(paint_tool_bg(&line, width, false, false, theme));
                }
                if fold.tools_expanded && !display_diff.is_empty() {
                    let input = DiffInput::DisplayText(display_diff.clone());
                    let opts = DiffOptions {
                        word_level: true,
                        ..DiffOptions::default()
                    };
                    for line in
                        render_diff_lines(&input, width, &theme.palette().diff_theme(), &opts)
                    {
                        lines.push(fit(&line, width));
                    }
                }
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
        lines.push(String::new());
    }

    for (kind, text) in model.streaming_scrollback_tails() {
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

    // Drop trailing blank from last entry spacer when present.
    while lines.last().is_some_and(|l| l.trim().is_empty()) {
        lines.pop();
    }
    lines
}
