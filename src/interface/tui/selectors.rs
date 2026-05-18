//! Selector overlays for the TUI.
//!
//! Provides session, model, and theme selection overlays.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

/// The kind of selector currently active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SelectorKind {
    Session,
    Model,
    Theme,
}

/// A generic list selector — re-used by session, model, and theme selectors.
pub(crate) struct ListSelector {
    /// Title of the selector.
    title: String,
    /// Available items.
    items: Vec<String>,
    /// Currently selected index.
    selected: usize,
    /// Whether the selector is active.
    active: bool,
    /// Callback result.
    result: Option<String>,
}

impl ListSelector {
    pub(crate) fn new(title: impl Into<String>, items: Vec<String>) -> Self {
        Self {
            title: title.into(),
            items,
            selected: 0,
            active: false,
            result: None,
        }
    }

    /// Show the selector.
    pub(crate) fn show(&mut self) {
        self.active = true;
        self.selected = 0;
        self.result = None;
    }

    pub(crate) fn is_active(&self) -> bool {
        self.active
    }

    pub(crate) fn select_prev(&mut self) {
        if self.active {
            self.selected = self.selected.saturating_sub(1);
        }
    }

    pub(crate) fn select_next(&mut self) {
        if self.active && self.selected + 1 < self.items.len() {
            self.selected += 1;
        }
    }

    /// Confirm selection.
    pub(crate) fn confirm(&mut self) -> Option<String> {
        if !self.active {
            return None;
        }
        let item = self.items[self.selected].clone();
        self.result = Some(item.clone());
        self.active = false;
        Some(item)
    }

    /// Cancel without selecting.
    pub(crate) fn cancel(&mut self) {
        self.active = false;
        self.result = None;
    }

    /// Get the result.
    pub(crate) fn result(&self) -> Option<&str> {
        self.result.as_deref()
    }

    /// Render the selector overlay.
    pub(crate) fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.active {
            return;
        }

        let overlay_w = (area.width as f32 * 0.5) as u16;
        let overlay_h = (self.items.len() as u16 + 4).min(area.height.saturating_sub(4));
        let x = (area.width - overlay_w) / 2;
        let y = (area.height - overlay_h) / 2;
        let overlay_rect = Rect::new(x, y, overlay_w.max(30), overlay_h.max(6));

        frame.render_widget(Clear, overlay_rect);

        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", self.title))
            .title_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .style(Style::default().bg(Color::Rgb(10, 10, 20)));
        let inner = block.inner(overlay_rect);
        frame.render_widget(block, overlay_rect);

        let mut lines = Vec::new();
        for (i, item) in self.items.iter().enumerate() {
            let selected = i == self.selected;
            let prefix = if selected { " > " } else { "   " };
            let style = if selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            lines.push(Line::from(ratatui::text::Span::styled(
                format!("{}{}", prefix, item),
                style,
            )));
        }

        let para = Paragraph::new(lines).wrap(Wrap { trim: false });
        frame.render_widget(para, inner);
    }
}

/// Create a session selector with default options.
pub(crate) fn session_selector() -> ListSelector {
    ListSelector::new(" Sessions ", vec!["default".into()])
}

/// Create a model selector with default options.
pub(crate) fn model_selector() -> ListSelector {
    ListSelector::new(" Select Model ", vec!["default".into()])
}

/// Create a theme selector with default options.
pub(crate) fn theme_selector() -> ListSelector {
    ListSelector::new(" Select Theme ", vec!["default".into()])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selector_navigation() {
        let mut sel = ListSelector::new("Test", vec!["a".into(), "b".into(), "c".into()]);
        sel.show();
        assert!(sel.is_active());
        sel.select_next();
        sel.select_next();
        let result = sel.confirm();
        assert_eq!(result.as_deref(), Some("c"));
        assert!(!sel.is_active());
    }

    #[test]
    fn test_selector_cancel() {
        let mut sel = ListSelector::new("Test", vec!["a".into()]);
        sel.show();
        sel.cancel();
        assert!(!sel.is_active());
        assert!(sel.result().is_none());
    }
}
