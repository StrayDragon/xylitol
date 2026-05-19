//! Selector overlay — generic list picker.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState};

use crate::interface::tui::component::{Component, EventResult, OverlayAction};
use crate::interface::tui::event::{AppAction, TuiEvent};

pub(crate) struct SelectorOverlay {
    title: String,
    items: Vec<String>,
    selected: usize,
    dirty: bool,
}

impl SelectorOverlay {
    pub(crate) fn new(title: impl Into<String>, items: Vec<String>) -> Self {
        Self {
            title: title.into(),
            items,
            selected: 0,
            dirty: true,
        }
    }

    fn centered(area: Rect, width: u16, height: u16) -> Rect {
        let w = width.min(area.width);
        let h = height.min(area.height);
        let x = area.x + (area.width.saturating_sub(w) / 2);
        let y = area.y + (area.height.saturating_sub(h) / 2);
        Rect::new(x, y, w, h)
    }
}

impl Component for SelectorOverlay {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let modal = Self::centered(area, 70, 18);
        frame.render_widget(Clear, modal);

        let items = self
            .items
            .iter()
            .map(|s| ListItem::new(s.as_str()))
            .collect::<Vec<_>>();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" {} ", self.title))
                    .border_style(Style::default().fg(Color::LightCyan)),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD),
            );

        let mut state = ListState::default();
        if !self.items.is_empty() {
            state.select(Some(self.selected.min(self.items.len().saturating_sub(1))));
        }

        frame.render_stateful_widget(list, modal, &mut state);
        self.dirty = false;
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn mark_clean(&mut self) {
        self.dirty = false;
    }

    fn handle_event(&mut self, event: &TuiEvent) -> EventResult {
        let TuiEvent::Key(key) = event else {
            return EventResult::default();
        };
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Esc => EventResult {
                consumed: true,
                overlay: Some(OverlayAction::Dismiss),
                ..EventResult::default()
            },
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected > 0 {
                    self.selected -= 1;
                    self.dirty = true;
                }
                EventResult::consumed()
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected + 1 < self.items.len() {
                    self.selected += 1;
                    self.dirty = true;
                }
                EventResult::consumed()
            }
            KeyCode::Enter => {
                let selected = self.items.get(self.selected).cloned().unwrap_or_default();
                EventResult {
                    consumed: true,
                    action: Some(AppAction::SetDiff(selected)),
                    overlay: Some(OverlayAction::Dismiss),
                }
            }
            _ => EventResult::consumed(),
        }
    }
}
