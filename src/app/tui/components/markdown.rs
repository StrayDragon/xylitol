//! `MarkdownRenderer` adapter — render finalized markdown into `Vec<Line>`
//! for the ratatui buffer (c355/c366/c370).
//!
//! Since c370, the actual parsing + rendering is backed by the vendored
//! ratatui-markdown core (`crate::app::tui::vendor::ratatui_markdown`), which
//! provides full CommonMark/GFM coverage (tables, task lists, nested
//! blockquotes) plus syntect code highlighting. This module is the thin
//! **style adapter**: it maps xylitol's `Palette` to the vendored `ThemeConfig`
//! (user/assistant/thinking each get their own theme) and wires the syntect
//! highlighter into the renderer's `RenderHooks`.
//!
//! Both user input and assistant replies share this same code path, differing
//! only in the injected [`MarkdownStyle`] (spec tui60). Streaming text MUST
//! NOT pass through here (spec tui61 streaming-stays-plain); only finalized
//! text committed to scrollback is rendered.

use std::sync::Arc;

use ratatui_core::style::Color;
use ratatui_core::text::Line;

use crate::app::tui::theme::Palette;
use crate::app::tui::vendor::ratatui_markdown::highlight::{
    CodeHighlighter, HighlightHooks, SyntectHighlighter,
};
use crate::app::tui::vendor::ratatui_markdown::markdown::MarkdownRenderer;
use crate::app::tui::vendor::ratatui_markdown::theme::ThemeConfig;

/// The style seam `RenderedLine::to_lines` consumes. Each variant maps to a
/// vendored `ThemeConfig` so user input, assistant replies, and thinking share
/// the same renderer and differ only in colors.
#[derive(Clone, Copy)]
pub struct MarkdownStyle {
    theme: ThemeConfig,
}

impl MarkdownStyle {
    /// Style for rendering user input (cyan/bold, matches `Palette::user_prompt`).
    pub fn for_user(_p: &Palette) -> Self {
        // Palette currently unused (fixed Cyan); kept for API symmetry + future tuning.
        let theme = ThemeConfig::default()
            .with_text_color(Color::Cyan)
            .with_primary_color(Color::Cyan);
        Self { theme }
    }

    /// Style for rendering assistant replies (default/reset, matches `Palette::assistant`).
    pub fn for_assistant(_p: &Palette) -> Self {
        // Default theme: normal-brightness text, standard code colors.
        Self {
            theme: ThemeConfig::default(),
        }
    }

    /// Style for rendering finalized thinking/reasoning text (c366). Same
    /// structural rendering as assistant (markdown parsed), but dimmed so
    /// reasoning stays visually subordinate to the reply.
    pub fn for_thinking(_p: &Palette) -> Self {
        // Dim everything: muted text + muted accents. Palette currently unused
        // (fixed DarkGray dim); kept in the signature for API symmetry with
        // for_user/for_assistant and future per-token tuning.
        let dim = Color::DarkGray;
        let theme = ThemeConfig::default()
            .with_text_color(dim)
            .with_muted_text_color(dim)
            .with_primary_color(dim);
        Self { theme }
    }
}

/// Render a finalized markdown string into styled `Line`s at `width`,
/// using the given `style` and syntect code highlighting (c370).
///
/// Streaming text MUST NOT pass through here (spec tui61 streaming-stays-plain);
/// only finalized text committed to scrollback is rendered.
pub fn render_markdown(text: &str, width: u16, style: &MarkdownStyle) -> Vec<Line<'static>> {
    let highlighter: Arc<dyn CodeHighlighter> = Arc::new(SyntectHighlighter::new());
    let hooks = HighlightHooks::new(highlighter, width as usize);
    let renderer = MarkdownRenderer::new(width as usize).with_render_hooks(Box::new(hooks));
    let blocks = renderer.parse(text);
    renderer.render(&blocks, &style.theme)
}

#[cfg(test)]
mod tests {
    //! Behavior assertions (spec tui41) for the markdown render path. Each
    //! test renders via `render_markdown` then draws to a TestBackend buffer
    //! and asserts row content — implementation-agnostic (the vendored
    //! renderer swap in c370 must keep these green).

    use super::*;
    use crate::app::tui::theme::palette;
    use ratatui_core::backend::TestBackend;
    use ratatui_core::buffer::CellWidth;
    use ratatui_core::layout::Rect;
    use ratatui_core::terminal::Terminal;
    use ratatui_core::widgets::Widget;

    fn render_to_buf(text: &str, width: u16) -> (ratatui_core::buffer::Buffer, u16) {
        let style = MarkdownStyle::for_assistant(&palette());
        let lines = render_markdown(text, width, &style);
        let height = lines.len() as u16;
        let mut term = Terminal::new(TestBackend::new(width, height.max(1))).unwrap();
        term.draw(|f| {
            let area = Rect::new(0, 0, width, height.max(1));
            f.render_widget(ratatui_widgets::paragraph::Paragraph::new(lines), area);
        })
        .unwrap();
        (term.backend().buffer().clone(), height)
    }

    fn row_text(buf: &ratatui_core::buffer::Buffer, y: u16, width: u16) -> String {
        let mut out = String::new();
        let mut prev_width: u16 = 1;
        for x in 0..width {
            let cell = &buf[(x, y)];
            let sym = cell.symbol();
            let is_filler =
                sym.is_empty() || (sym == " " && prev_width == 2) || cell.cell_width() == 0;
            if is_filler {
                continue;
            }
            if let Some(ch) = sym.chars().next() {
                out.push(ch);
            }
            prev_width = cell.cell_width().max(1) as u16;
        }
        out.trim_end().to_string()
    }

    #[test]
    fn heading_renders() {
        let (buf, _) = render_to_buf("# Title", 40);
        let row = row_text(&buf, 0, 40);
        assert!(row.contains("Title"), "heading text visible: {row}");
    }

    #[test]
    fn bold_inline_renders() {
        let (buf, _) = render_to_buf("some **bold** word", 40);
        let row = row_text(&buf, 0, 40);
        assert!(row.contains("bold"), "bold text visible: {row}");
    }

    #[test]
    fn inline_code_renders() {
        let (buf, _) = render_to_buf("use `cargo` now", 40);
        let row = row_text(&buf, 0, 40);
        assert!(row.contains("cargo"), "inline code visible: {row}");
    }

    #[test]
    fn fenced_code_block_has_language_label() {
        let md = "```rs\nfn main() {}\n```\n";
        let (buf, h) = render_to_buf(md, 40);
        // The vendored renderer draws a bordered code block; the language label
        // appears on the header row. Find any row mentioning the language.
        let found = (0..h).any(|y| row_text(&buf, y, 40).contains("rs"));
        assert!(found, "language label 'rs' present somewhere: rows 0..{h}");
    }

    #[test]
    fn code_block_highlights_rust() {
        // spec tui71: recognized language gets non-default styling on code spans.
        let style = MarkdownStyle::for_assistant(&palette());
        let lines = render_markdown("```rs\nfn main() {}\n```\n", 40, &style);
        let any_styled = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .any(|s| s.style.fg.is_some());
        assert!(
            any_styled,
            "some code span has foreground color (highlighted)"
        );
    }

    #[test]
    fn unordered_list_renders_marker() {
        let md = "- first\n- second\n";
        let (buf, h) = render_to_buf(md, 40);
        assert!(h >= 2, "two list items -> at least 2 rows, got {h}");
        let row0 = row_text(&buf, 0, 40);
        assert!(row0.contains("first"), "first item text visible: {row0}");
    }

    #[test]
    fn blockquote_renders() {
        // spec tui66: blockquote has a visible prefix (the vendored renderer
        // uses a `│` left-bar; c366's `▎` was superseded by the richer
        // vendored blockquote rendering in c370).
        let md = "> a quote\n";
        let (buf, _) = render_to_buf(md, 40);
        let row = row_text(&buf, 0, 40);
        assert!(row.contains("a quote"), "quote body visible: {row}");
        assert!(
            row.contains('│') || row.contains('▎'),
            "blockquote prefix visible: {row}"
        );
    }

    #[test]
    fn table_renders_aligned() {
        // spec tui61 table-renders (c370 adds table support).
        let md = "| a | b |\n|---|---|\n| 1 | 2 |\n";
        let (buf, h) = render_to_buf(md, 40);
        let all_text: String = (0..h)
            .map(|y| row_text(&buf, y, 40))
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            all_text.contains('a') && all_text.contains('b'),
            "headers: {all_text}"
        );
        assert!(
            all_text.contains('1') && all_text.contains('2'),
            "cells: {all_text}"
        );
    }

    #[test]
    fn unknown_lang_falls_back_to_plain() {
        // spec tui71 unrecognized-language-fallback: no panic, plain render.
        let md = "```unknownlang\ncode here\n```\n";
        let (buf, h) = render_to_buf(md, 40);
        let found = (0..h).any(|y| row_text(&buf, y, 40).contains("code here"));
        assert!(found, "unknown-lang code still renders as plain text");
    }

    #[test]
    fn user_vs_assistant_share_renderer() {
        let p = palette();
        let user_lines = render_markdown("# t", 40, &MarkdownStyle::for_user(&p));
        let asst_lines = render_markdown("# t", 40, &MarkdownStyle::for_assistant(&p));
        let user_text: String = user_lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.to_string()))
            .collect();
        let asst_text: String = asst_lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.to_string()))
            .collect();
        assert_eq!(
            user_text, asst_text,
            "user/assistant share structural rendering (only styles differ)"
        );
    }

    #[test]
    fn thinking_text_renders_markdown() {
        // spec tui65: thinking renders through markdown (code block parsed,
        // not flat). The for_thinking style dims it but structure is preserved.
        let p = palette();
        let lines = render_markdown(
            "```\nfn think() {}\n```\n",
            40,
            &MarkdownStyle::for_thinking(&p),
        );
        let text: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.to_string()))
            .collect();
        assert!(
            text.contains("fn think()"),
            "thinking code block parsed (not flat): {text}"
        );
    }

    #[test]
    fn nested_blockquote_indents() {
        // spec tui66 nested-blockquote: deeper nesting carries more prefix.
        let md = ">> nested\n";
        let (buf, _) = render_to_buf(md, 40);
        let row = row_text(&buf, 0, 40);
        // Nested quote has at least 2 prefix glyphs.
        let prefix_count = row.chars().filter(|&c| c == '│' || c == '▎').count();
        assert!(
            prefix_count >= 2,
            "nested blockquote has >=2 prefix glyphs, got {prefix_count}: {row}"
        );
    }

    #[test]
    fn inline_link_renders_text_and_url() {
        // spec tui72: [text](url) -> "text (url)" so URL is visible + copyable.
        let md = "see [GitHub](https://github.com) now";
        let (buf, _) = render_to_buf(md, 60);
        let row = row_text(&buf, 0, 60);
        assert!(row.contains("GitHub"), "link text visible: {row}");
        assert!(
            row.contains("https://github.com"),
            "url visible for copy: {row}"
        );
        assert!(row.contains('(') && row.contains(')'), "parentheses: {row}");
    }

    #[test]
    fn autolink_renders_bare_url() {
        // spec tui72: <url> -> bare url (no angle brackets).
        let md = "visit <https://example.com> today";
        let (buf, _) = render_to_buf(md, 60);
        let row = row_text(&buf, 0, 60);
        assert!(
            row.contains("https://example.com"),
            "autolink url visible: {row}"
        );
        assert!(
            !row.contains('<') && !row.contains('>'),
            "no angle brackets: {row}"
        );
    }

    #[test]
    fn empty_link_text_just_url() {
        // spec tui72: [](url) -> just the url.
        let md = "[](https://bare.example)";
        let (buf, _) = render_to_buf(md, 60);
        let row = row_text(&buf, 0, 60);
        assert!(
            row.contains("https://bare.example"),
            "url visible for empty link text: {row}"
        );
    }
}
