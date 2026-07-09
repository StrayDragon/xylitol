use crate::tui::Component;
use crate::utils::{truncate_to_width, visible_width};

/// Text component that truncates to fit viewport width (single line).
pub struct TruncatedText {
    text: String,
    padding_x: usize,
    padding_y: usize,
}

impl TruncatedText {
    pub fn new(text: String, padding_x: usize, padding_y: usize) -> Self {
        Self {
            text,
            padding_x,
            padding_y,
        }
    }
}

impl Component for TruncatedText {
    fn render(&mut self, width: usize) -> Vec<String> {
        let mut result = Vec::new();
        let empty_line = " ".repeat(width);

        // Top padding
        for _ in 0..self.padding_y {
            result.push(empty_line.clone());
        }

        let available = std::cmp::max(1, width.saturating_sub(self.padding_x * 2));

        // Take only first line
        let single_line = match self.text.find('\n') {
            Some(nl) => &self.text[..nl],
            None => &self.text,
        };

        let display_text = truncate_to_width(single_line, available, "...", false);

        let left_pad = " ".repeat(self.padding_x);
        let right_pad = " ".repeat(self.padding_x);
        let with_padding = format!("{}{}{}", left_pad, display_text, right_pad);
        let vis = visible_width(&with_padding);
        let pad_needed = width.saturating_sub(vis);
        result.push(format!("{}{}", with_padding, " ".repeat(pad_needed)));

        // Bottom padding
        for _ in 0..self.padding_y {
            result.push(empty_line.clone());
        }

        if result.is_empty() {
            vec![String::new()]
        } else {
            result
        }
    }

    fn handle_input(&mut self, _event: crate::tui::InputEvent) {}
    fn invalidate(&mut self) {}
}
