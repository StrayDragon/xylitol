//! Markdown style adapter (c395) — thin seam over the self-researched
//! renderer (`markdown_render`).
#![allow(clippy::doc_lazy_continuation)]
//!
//! `MarkdownStyle` maps each role (user/assistant/thinking) to a
//! `markdown_render::RenderStyle`. The actual parsing + rendering (pulldown-cmark
//! + syntect, no borders/tree-connectors) lives in `markdown_render`. This
//! module keeps the `render_markdown(text, width, &MarkdownStyle) -> Vec<Line>`
//! signature stable so `render.rs` and its behavior tests are unaffected by
//! the renderer swap.
//!
//! Both user input and assistant replies share this same code path, differing
//! only in the injected [`MarkdownStyle`] (spec tui60). Streaming text MUST
//! NOT pass through here (spec tui61 streaming-stays-plain); only finalized
//! text committed to scrollback is rendered.

use ratatui_core::text::Line;

use crate::app::tui::components::markdown_render::RenderStyle;
use crate::app::tui::theme::Palette;

/// The style seam `RenderedLine::to_lines` consumes. Each variant maps to a
/// `RenderStyle` so user input, assistant replies, and thinking share the same
/// self-researched renderer and differ only in colors.
#[derive(Clone, Copy)]
pub struct MarkdownStyle {
    rs: RenderStyle,
}

impl MarkdownStyle {
    /// Style for rendering user input (cyan/bold, matches `Palette::user_prompt`).
    pub fn for_user(_p: &Palette) -> Self {
        Self {
            rs: RenderStyle::for_user(),
        }
    }

    /// Style for rendering assistant replies (default/reset).
    pub fn for_assistant(_p: &Palette) -> Self {
        Self {
            rs: RenderStyle::for_assistant(),
        }
    }

    /// Style for rendering finalized thinking/reasoning text (c366). Same
    /// structural rendering as assistant (markdown parsed), but dimmed so
    /// reasoning stays visually subordinate to the reply.
    pub fn for_thinking(_p: &Palette) -> Self {
        Self {
            rs: RenderStyle::for_thinking(),
        }
    }
}

/// Render a finalized markdown string into styled `Line`s at `width`,
/// using the given `style` and syntect code highlighting (c395: self-researched
/// renderer — no borders, terminal-friendly styling only).
///
/// Streaming text MUST NOT pass through here (spec tui61 streaming-stays-plain);
/// only finalized text committed to scrollback is rendered.
pub fn render_markdown(text: &str, width: u16, style: &MarkdownStyle) -> Vec<Line<'static>> {
    crate::app::tui::components::markdown_render::render_markdown(text, width, style.rs)
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
    fn fenced_code_block_renders_content() {
        // c371 follow-up: code blocks render with syntax highlighting only —
        // no border frame (╭─/╰─) and no language label. The code body is the
        // only content, visually set off by surrounding blank lines.
        let md = "```rs\nfn main() {}\n```\n";
        let (buf, h) = render_to_buf(md, 40);
        let found = (0..h).any(|y| row_text(&buf, y, 40).contains("fn main()"));
        assert!(found, "code body renders: rows 0..{h}");
        // No border frame glyphs anywhere.
        for y in 0..h {
            let row = row_text(&buf, y, 40);
            assert!(
                !row.contains('╭') && !row.contains('╰') && !row.contains('│'),
                "no border frame on row {y}: {row}"
            );
        }
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
        // c396: blockquote renders italic + dim WITH a "> " prefix (spec tui66).
        let md = "> a quote\n";
        let (buf, _) = render_to_buf(md, 40);
        let row = row_text(&buf, 0, 40);
        assert!(row.contains("a quote"), "quote body visible: {row}");
        assert!(
            row.starts_with("> "),
            "blockquote has > prefix (c396): {row}"
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
    fn nested_blockquote_renders() {
        // c396: nested blockquote renders with a doubled "> > " prefix.
        let md = ">> nested\n";
        let (buf, _) = render_to_buf(md, 40);
        let row = row_text(&buf, 0, 40);
        assert!(row.contains("nested"), "nested quote body: {row}");
        assert!(
            row.starts_with("> > "),
            "nested blockquote has doubled prefix (c396): {row}"
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
