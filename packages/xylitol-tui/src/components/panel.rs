use crate::tui::Component;
use crate::utils::{BgFn, apply_background_to_line, visible_width};

/// Panel component - a container that applies padding and background to all children.
///
/// Renders children sequentially within a padded area, with optional background color
/// applied to every line. Uses a render cache keyed on width, child output, and background.
pub struct Panel {
    children: Vec<Box<dyn Component>>,
    padding_x: usize,
    padding_y: usize,
    bg_fn: Option<BgFn>,
    cache: Option<PanelCache>,
}

struct PanelCache {
    child_lines: Vec<String>,
    width: usize,
    bg_sample: Option<String>,
    lines: Vec<String>,
}

impl Panel {
    pub fn new(padding_x: usize, padding_y: usize, bg_fn: Option<BgFn>) -> Self {
        Self {
            children: Vec::new(),
            padding_x,
            padding_y,
            bg_fn,
            cache: None,
        }
    }

    pub fn add_child(&mut self, component: Box<dyn Component>) {
        self.children.push(component);
        self.invalidate_cache();
    }

    pub fn remove_child(&mut self, index: usize) {
        if index < self.children.len() {
            self.children.remove(index);
            self.invalidate_cache();
        }
    }

    pub fn clear(&mut self) {
        self.children.clear();
        self.invalidate_cache();
    }

    fn invalidate_cache(&mut self) {
        self.cache = None;
    }

    fn match_cache(
        &self,
        width: usize,
        child_lines: &[String],
        bg_sample: &Option<String>,
    ) -> bool {
        match &self.cache {
            Some(c) => {
                c.width == width
                    && c.bg_sample == *bg_sample
                    && c.child_lines.len() == child_lines.len()
                    && c.child_lines.iter().zip(child_lines).all(|(a, b)| a == b)
            }
            None => false,
        }
    }
}

impl Component for Panel {
    fn render(&mut self, width: usize) -> Vec<String> {
        if self.children.is_empty() {
            return vec![];
        }

        let content_width = std::cmp::max(1, width.saturating_sub(self.padding_x * 2));
        let left_pad = " ".repeat(self.padding_x);

        // Render all children into flat line list with left padding
        let mut child_lines: Vec<String> = Vec::new();
        for child in &mut self.children {
            let lines = child.render(content_width);
            for line in lines {
                child_lines.push(format!("{}{}", left_pad, line));
            }
        }

        if child_lines.is_empty() {
            return vec![];
        }

        // Sample bgFn output to detect changes
        let bg_sample: Option<String> = self.bg_fn.as_ref().map(|f| f("test"));

        // Check cache
        if self.match_cache(width, &child_lines, &bg_sample) {
            return self.cache.as_ref().unwrap().lines.clone();
        }

        // Apply background and padding
        let mut result: Vec<String> = Vec::new();

        // Top padding
        for _ in 0..self.padding_y {
            result.push(self.apply_bg("", width));
        }

        // Content
        for line in &child_lines {
            result.push(self.apply_bg(line, width));
        }

        // Bottom padding
        for _ in 0..self.padding_y {
            result.push(self.apply_bg("", width));
        }

        self.cache = Some(PanelCache {
            child_lines,
            width,
            bg_sample,
            lines: result.clone(),
        });

        result
    }

    fn handle_input(&mut self, event: crate::tui::InputEvent) {
        for child in &mut self.children {
            child.handle_input(event.clone());
        }
    }

    fn invalidate(&mut self) {
        self.invalidate_cache();
        for child in &mut self.children {
            child.invalidate();
        }
    }
}

impl Panel {
    fn apply_bg(&self, line: &str, width: usize) -> String {
        let vis_len = visible_width(line);
        let pad_needed = width.saturating_sub(vis_len);
        let padded = format!("{}{}", line, " ".repeat(pad_needed));

        if let Some(ref bg) = self.bg_fn {
            apply_background_to_line(&padded, width, bg.as_ref())
        } else {
            padded
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::spacer::Spacer;
    use crate::components::text::Text;

    #[test]
    fn test_panel_empty_no_children() {
        let mut panel = Panel::new(1, 1, None);
        let lines = panel.render(80);
        assert!(lines.is_empty());
    }

    #[test]
    fn test_panel_padding_applied() {
        let mut panel = Panel::new(1, 1, None);
        panel.add_child(Box::new(Text::new("hello".into(), 0, 0)));

        let lines = panel.render(10);
        // Top padding (1), content " hello" padded to 10, bottom padding (1) = 3 lines
        assert_eq!(lines.len(), 3);
        // Content line has 1 space left pad + "hello" + 4 spaces right pad = 10
        assert_eq!(lines[1].len(), 10);
        assert!(lines[1].starts_with(" hello"));
    }

    #[test]
    fn test_panel_horizontal_padding() {
        let mut panel = Panel::new(2, 0, None);
        panel.add_child(Box::new(Text::new("hi".into(), 0, 0)));

        let lines = panel.render(8);
        // One content line: "  hi" + pad
        assert_eq!(lines.len(), 1);
        assert!(lines[0].starts_with("  hi"));
    }

    #[test]
    fn test_panel_background_applied() {
        let mut panel = Panel::new(
            1,
            0,
            Some(Box::new(|s: &str| format!("\x1b[44m{}\x1b[49m", s))),
        );
        panel.add_child(Box::new(Text::new("x".into(), 0, 0)));

        let lines = panel.render(10);
        assert!(lines[0].contains("\x1b[44m"));
        assert!(lines[0].contains("\x1b[49m"));
    }

    #[test]
    fn test_panel_multiple_children() {
        let mut panel = Panel::new(1, 0, None);
        panel.add_child(Box::new(Text::new("first".into(), 0, 0)));
        panel.add_child(Box::new(Text::new("second".into(), 0, 0)));

        let lines = panel.render(10);
        // Both children rendered with left padding
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("first"));
        assert!(lines[1].contains("second"));
    }

    #[test]
    fn test_panel_cache_hit() {
        let mut panel = Panel::new(1, 0, None);
        panel.add_child(Box::new(Text::new("cached".into(), 0, 0)));

        // First render caches
        let lines1 = panel.render(20);
        // Second render with same width and content returns cached
        let lines2 = panel.render(20);
        assert_eq!(lines1, lines2);
    }

    #[test]
    fn test_panel_cache_invalidates_on_width_change() {
        let mut panel = Panel::new(1, 0, None);
        panel.add_child(Box::new(Text::new("test".into(), 0, 0)));

        let lines1 = panel.render(20);
        let lines2 = panel.render(40);
        // Different widths should produce different output
        assert_ne!(lines1, lines2);
    }

    #[test]
    fn test_panel_clear() {
        let mut panel = Panel::new(1, 1, None);
        panel.add_child(Box::new(Text::new("text".into(), 0, 0)));
        let before = panel.render(80);
        assert!(!before.is_empty());

        panel.clear();
        let after = panel.render(80);
        assert!(after.is_empty());
    }

    #[test]
    fn test_panel_remove_child() {
        let mut panel = Panel::new(1, 0, None);
        panel.add_child(Box::new(Text::new("keep".into(), 0, 0)));
        panel.add_child(Box::new(Text::new("remove".into(), 0, 0)));

        let before = panel.render(80);
        assert_eq!(before.len(), 2);

        panel.remove_child(1);
        let after = panel.render(80);
        assert_eq!(after.len(), 1);
    }

    #[test]
    fn test_panel_no_padding() {
        let mut panel = Panel::new(0, 0, None);
        panel.add_child(Box::new(Text::new("zero".into(), 0, 0)));

        let lines = panel.render(10);
        assert_eq!(lines.len(), 1);
        // No left padding, text starts immediately
        assert!(lines[0].starts_with("zero"));
    }

    #[test]
    fn test_panel_vertical_padding_only() {
        let mut panel = Panel::new(0, 2, None);
        panel.add_child(Box::new(Text::new("content".into(), 0, 0)));

        let lines = panel.render(10);
        // 2 top + 1 content + 2 bottom = 5
        assert_eq!(lines.len(), 5);
        // Top padding lines are empty (spaces padded to width)
        assert!(lines[0].trim().is_empty());
        assert!(lines[1].trim().is_empty());
        assert_eq!(lines[2].trim(), "content");
        assert!(lines[3].trim().is_empty());
        assert!(lines[4].trim().is_empty());
    }

    #[test]
    fn test_panel_child_with_spacer() {
        let mut panel = Panel::new(1, 0, None);
        panel.add_child(Box::new(Text::new("a".into(), 0, 0)));
        panel.add_child(Box::new(Spacer::new(2)));
        panel.add_child(Box::new(Text::new("b".into(), 0, 0)));

        let lines = panel.render(10);
        // text "a" + 2 empty spacer lines + text "b" = 4 lines
        assert_eq!(lines.len(), 4);
        assert!(lines[0].contains("a"));
        assert!(lines[1].trim().is_empty());
        assert!(lines[2].trim().is_empty());
        assert!(lines[3].contains("b"));
    }
}
