mod config;
mod hooks;
mod segment;
mod syntect_bridge;

pub use config::{HIGHLIGHT_NAMES, highlight_to_style};
pub use hooks::HighlightHooks;
use ratatui_core::{style::Style, text::Line};
pub use segment::segments_to_lines;
pub use syntect_bridge::SyntectHighlighter;

#[derive(Debug, Clone)]
pub struct StyleSegment {
    pub start: usize,
    pub end: usize,
    pub style: Style,
}

pub trait CodeHighlighter: Send + Sync {
    fn highlight(&self, lang: &str, code: &str) -> Vec<StyleSegment>;
}

pub fn highlight_to_lines(
    highlighter: &dyn CodeHighlighter,
    lang: &str,
    code: &str,
    prefix: &str,
    border_style: Style,
    max_width: usize,
) -> Vec<Line<'static>> {
    let code = code.replace('\t', "    ");
    let segments = highlighter.highlight(lang, &code);
    segment::segments_to_lines(&code, &segments, prefix, border_style, max_width)
}
