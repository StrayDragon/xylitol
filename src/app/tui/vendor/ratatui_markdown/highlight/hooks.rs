use std::sync::Arc;

use ratatui_core::style::Style;
use ratatui_core::text::Line;

use super::CodeHighlighter;

pub struct HighlightHooks {
    highlighter: Arc<dyn CodeHighlighter>,
    max_width: usize,
}

impl HighlightHooks {
    pub fn new(highlighter: Arc<dyn CodeHighlighter>, max_width: usize) -> Self {
        Self {
            highlighter,
            max_width,
        }
    }
}

impl crate::app::tui::vendor::ratatui_markdown::markdown::RenderHooks for HighlightHooks {
    fn render_code_block(&self, lang: &str, content: &str) -> Option<Vec<Line<'static>>> {
        let segments = self.highlighter.highlight(lang, content);
        if segments.is_empty() {
            return None;
        }

        // c371 follow-up: render code with syntax highlighting only — no border
        // frame (╭─/│/╰─) and no language label. The code is visually set off
        // by its highlighting + surrounding blank lines, like a plain editor.
        let content_width = self.max_width;
        let code_lines = super::segment::segments_to_lines(
            content,
            &segments,
            "",
            Style::default(),
            content_width,
        );
        Some(code_lines)
    }
}
