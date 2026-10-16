use crate::tui::Component;
use crate::utils::{BgFn, apply_background_to_line, visible_width, wrap_text_with_ansi};

/// Text component - displays multi-line text with word wrapping.
pub struct Text {
    text: String,
    padding_x: usize,
    padding_y: usize,
    custom_bg_fn: Option<BgFn>,
    cache: Option<(String, usize, Vec<String>)>,
}

impl Text {
    pub fn new(text: String, padding_x: usize, padding_y: usize) -> Self {
        Self {
            text,
            padding_x,
            padding_y,
            custom_bg_fn: None,
            cache: None,
        }
    }

    pub fn set_text(&mut self, text: String) {
        self.text = text;
        self.cache = None;
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}

impl Component for Text {
    fn render(&mut self, width: usize) -> Vec<String> {
        if let Some((cached_text, cached_width, cached_lines)) = &self.cache
            && cached_text == &self.text
            && *cached_width == width
        {
            return cached_lines.clone();
        }

        if self.text.is_empty() || self.text.trim().is_empty() {
            let result: Vec<String> = Vec::new();
            self.cache = Some((self.text.clone(), width, result.clone()));
            return result;
        }

        let normalized = self.text.replace('\t', "   ");
        let content_width = std::cmp::max(1, width.saturating_sub(self.padding_x * 2));
        let wrapped = wrap_text_with_ansi(&normalized, content_width);

        let left_margin = " ".repeat(self.padding_x);
        let right_margin = " ".repeat(self.padding_x);
        let mut content_lines: Vec<String> = Vec::new();

        for line in &wrapped {
            let with_margins = format!("{}{}{}", left_margin, line, right_margin);
            if let Some(ref bg) = self.custom_bg_fn {
                content_lines.push(apply_background_to_line(&with_margins, width, bg.as_ref()));
            } else {
                let vis_len = visible_width(&with_margins);
                let pad = width.saturating_sub(vis_len);
                content_lines.push(format!("{}{}", with_margins, " ".repeat(pad)));
            }
        }

        let empty_line = " ".repeat(width);
        let mut empty_lines: Vec<String> = Vec::new();
        for _ in 0..self.padding_y {
            if let Some(ref bg) = self.custom_bg_fn {
                empty_lines.push(apply_background_to_line(&empty_line, width, bg.as_ref()));
            } else {
                empty_lines.push(empty_line.clone());
            }
        }

        let mut result = Vec::new();
        result.extend(empty_lines.clone());
        result.extend(content_lines);
        result.extend(empty_lines);

        let result = if result.is_empty() {
            vec![String::new()]
        } else {
            result
        };
        self.cache = Some((self.text.clone(), width, result.clone()));
        result
    }

    fn handle_input(&mut self, _event: crate::tui::InputEvent) {}
    fn invalidate(&mut self) {
        self.cache = None;
    }
}
