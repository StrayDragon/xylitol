use crate::tui::{Component, InputEvent};
use crate::utils::{truncate_to_width, visible_width};

type PanelPaint = Box<dyn Fn(&str) -> String>;

pub struct BorderedPanel {
    title: String,
    lines: Vec<String>,
    viewport: usize,
    scroll: usize,
    border_paint: Option<PanelPaint>,
    title_paint: Option<PanelPaint>,
}

impl BorderedPanel {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            lines: Vec::new(),
            viewport: 0,
            scroll: 0,
            border_paint: None,
            title_paint: None,
        }
    }

    pub fn set_border_paint(&mut self, paint: impl Fn(&str) -> String + 'static) {
        self.border_paint = Some(Box::new(paint));
    }

    pub fn set_title_paint(&mut self, paint: impl Fn(&str) -> String + 'static) {
        self.title_paint = Some(Box::new(paint));
    }

    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
    }

    pub fn set_lines(&mut self, lines: Vec<String>) {
        self.lines = lines;
    }

    pub fn set_viewport(&mut self, rows: usize) {
        self.viewport = rows;
    }

    pub fn viewport(&self) -> usize {
        self.viewport
    }

    pub fn content_rows(&self) -> usize {
        self.lines.len()
    }

    pub fn scroll(&self) -> usize {
        self.scroll
    }

    pub fn set_scroll(&mut self, scroll: usize) {
        self.scroll = scroll;
    }

    pub fn max_scroll(&self) -> usize {
        self.lines.len().saturating_sub(self.viewport)
    }

    pub fn scroll_by(&mut self, delta: isize) -> bool {
        let next = if delta < 0 {
            self.scroll.saturating_sub(delta.unsigned_abs())
        } else {
            self.scroll.saturating_add(delta as usize)
        };
        let clamped = next.min(self.max_scroll());
        let changed = clamped != self.scroll;
        self.scroll = clamped;
        changed
    }

    fn render_inner(&mut self, width: usize) -> Vec<String> {
        if width < 8 || self.lines.is_empty() {
            return Vec::new();
        }
        let visible = self.viewport.min(self.lines.len());
        if visible == 0 {
            return Vec::new();
        }
        let max_scroll = self.lines.len().saturating_sub(visible);
        if self.scroll > max_scroll {
            self.scroll = max_scroll;
        }
        let content_w = width.saturating_sub(4).max(1);
        let inner_w = width.saturating_sub(2);
        let max_title_w = width.saturating_sub(5).max(1);
        let title = truncate_to_width(&self.title, max_title_w, "…", false);
        let title_w = visible_width(&title);
        let fill = inner_w.saturating_sub(title_w + 3);
        let border = |text: &str| {
            self.border_paint
                .as_ref()
                .map(|paint| paint(text))
                .unwrap_or_else(|| text.to_string())
        };
        let painted_title = self
            .title_paint
            .as_ref()
            .map(|paint| paint(&title))
            .unwrap_or_else(|| title.clone());
        let mut out = Vec::with_capacity(visible + 2);
        out.push(format!(
            "{} {}{}{}",
            border("╭─"),
            painted_title,
            border(" "),
            border(&format!("{}╮", "─".repeat(fill)))
        ));
        for line in self.lines.iter().skip(self.scroll).take(visible) {
            let line = truncate_to_width(line, content_w, "…", false);
            let pad = content_w.saturating_sub(visible_width(&line));
            out.push(format!(
                "{} {line}{} {}",
                border("│"),
                " ".repeat(pad),
                border("│")
            ));
        }
        out.push(border(&format!("╰{}╯", "─".repeat(inner_w))));
        out
    }
}

impl Component for BorderedPanel {
    fn render(&mut self, width: usize) -> Vec<String> {
        self.render_inner(width)
    }

    fn handle_input(&mut self, _event: InputEvent) {}

    fn invalidate(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::Component;

    #[test]
    fn wraps_content_height_and_border() {
        let mut panel = BorderedPanel::new("Keys");
        panel.set_lines(vec!["a".into(), "b".into(), "c".into()]);
        panel.set_viewport(10);
        let lines = panel.render(20);
        assert_eq!(lines.len(), 5);
        assert!(lines[0].contains("Keys"));
        assert!(lines[1].contains('a'));
        assert!(lines[4].starts_with('╰'));
    }

    #[test]
    fn clamps_scroll_to_last_page() {
        let mut panel = BorderedPanel::new("Keys");
        panel.set_lines(vec!["a".into(), "b".into(), "c".into()]);
        panel.set_viewport(2);
        panel.set_scroll(9);
        let lines = panel.render(20);
        assert_eq!(lines.len(), 4);
        assert_eq!(panel.scroll(), 1);
        assert!(lines[1].contains('b'));
    }

    #[test]
    fn paints_border_and_title_separately() {
        let mut panel = BorderedPanel::new("Keys");
        panel.set_lines(vec!["a".into()]);
        panel.set_viewport(1);
        panel.set_border_paint(|text| format!("<b>{text}</b>"));
        panel.set_title_paint(|text| format!("<t>{text}</t>"));
        let lines = panel.render(20);
        assert!(lines[0].contains("<t>Keys</t>"));
        assert!(lines[0].contains("<b>"));
        assert!(lines[1].contains("<b>"));
    }
}
