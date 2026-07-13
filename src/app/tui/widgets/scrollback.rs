//! Live scrollback rendering — Markdown / Expandable / Diff (c476 / c668).
//!
//! Morphology SSOT: `agent_demo` + `design/{markdown,expandable,diff-block,bash-mode}.md`.
//! Not a Codex TranscriptView — lines go into the engine scrollback stack.

use xylitol_tui::{
    Component, DiffInput, DiffOptions, ExpandableOutputOptions, Markdown, TruncateFrom,
    apply_background_to_line, bg_rgb, render_diff_lines, render_expandable_output,
    truncate_to_width, visible_width, wrap_text_with_ansi,
};

use super::glyphs::GlyphSet;
use crate::app::tui::bridge::{BashBlockStatus, UiEntry, UiModel};
use crate::app::tui::layout::LayoutTheme;
use xylitol_tui::terminal_colors::RgbColor;

/// Fold state owned by the product surface (att7).
#[derive(Debug, Clone, Copy, Default)]
pub struct ScrollbackFold {
    pub thinking_expanded: bool,
    /// Alt+E — tool/diff **block** show/hide detail.
    pub tools_expanded: bool,
    /// Ctrl+O — tool/bash detail **viewport** collapsed ↔ full (orthogonal to Alt+E).
    pub tools_output_expanded: bool,
}

/// Max visual lines for collapsed tool/bash detail (pi bash tool = 5).
const TOOLS_OUTPUT_PREVIEW_LINES: usize = 5;

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

    let mut need_spacer = false;
    for entry in &model.entries {
        if need_spacer {
            lines.push(inter_block_spacer(width));
        }
        need_spacer = true;
        match entry {
            UiEntry::User { text } => {
                let prefix = theme.paint_user(glyphs.user());
                let body = format!("{prefix} {text}");
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
                        max_preview_lines: TOOLS_OUTPUT_PREVIEW_LINES,
                        from: TruncateFrom::Tail,
                        expand_hint: "ctrl+o to expand".into(),
                        hint_style: None,
                    };
                    for line in
                        render_expandable_output(output, width, fold.tools_output_expanded, &opts)
                    {
                        block.push(line);
                    }
                }
                let rgb = tool_bg_rgb(!done, *is_error, theme);
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
                let header = format!(
                    "{marker} {} {summary}  {}",
                    glyphs.tool(),
                    key_hint("Alt+E")
                );
                let mut header_lines = Vec::new();
                push_wrapped(&mut header_lines, &theme.paint_tool(&header), width);
                let rgb = tool_bg_rgb(false, false, theme);
                push_tinted(&mut lines, &header_lines, width, rgb);
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
                    let body =
                        if matches!(status, BashBlockStatus::Error | BashBlockStatus::Cancelled) {
                            theme.paint_error(output)
                        } else {
                            theme.paint_muted(output)
                        };
                    let opts = ExpandableOutputOptions {
                        max_preview_lines: TOOLS_OUTPUT_PREVIEW_LINES,
                        from: TruncateFrom::Tail,
                        expand_hint: "ctrl+o to expand".into(),
                        hint_style: None,
                    };
                    for line in
                        render_expandable_output(&body, width, fold.tools_output_expanded, &opts)
                    {
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
