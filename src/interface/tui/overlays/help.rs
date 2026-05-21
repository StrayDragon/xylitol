//! Help overlay — keyboard reference.

use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::interface::tui::component::{Component, EventResult, OverlayAction};
use crate::interface::tui::event::TuiEvent;

pub(crate) struct HelpOverlay {
    dirty: bool,
}

impl HelpOverlay {
    pub(crate) fn new() -> Self {
        Self { dirty: true }
    }

    fn centered(area: Rect, width: u16, height: u16) -> Rect {
        let w = width.min(area.width);
        let h = height.min(area.height);
        let x = area.x + (area.width.saturating_sub(w) / 2);
        let y = area.y + (area.height.saturating_sub(h) / 2);
        Rect::new(x, y, w, h)
    }
}

impl Component for HelpOverlay {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let modal = Self::centered(area, 70, 18);
        frame.render_widget(Clear, modal);

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Help ")
            .border_style(Style::default().fg(Color::LightCyan));

        let lines = vec![
            Line::from(vec![
                Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" / "),
                Span::styled("?", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" — close help"),
            ]),
            Line::from(""),
            Line::from("Navigation"),
            Line::from("  Tab          queue (running) / submit (idle, except '!')"),
            Line::from("  Shift+Tab    switch focus (input/chat)"),
            Line::from("  j/k, ↑/↓     scroll chat (when focused)"),
            Line::from("  g / G        top / bottom"),
            Line::from(""),
            Line::from("Input"),
            Line::from("  Enter        submit"),
            Line::from("  Shift+Enter  newline"),
            Line::from("  Esc          cancel draft / backtrack"),
            Line::from("  Ctrl+K/U/W   kill-line / kill-backward / delete-word"),
            Line::from("  Ctrl+A/E     line start / end"),
            Line::from(""),
            Line::from("Runtime"),
            Line::from("  Ctrl+C       interrupt running agent"),
            Line::from("  Ctrl+L       clear chat"),
            Line::from("  Ctrl+T       transcript"),
            Line::from("  Ctrl+O       copy last response"),
            Line::from("  Alt+Y        view thinking"),
            Line::from("  Alt+R        raw output"),
            Line::from("  Ctrl+D       quit"),
        ];

        let para = Paragraph::new(lines)
            .block(block)
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(Color::Reset));
        frame.render_widget(para, modal);

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
            KeyCode::Esc | KeyCode::Char('?') => EventResult {
                consumed: true,
                overlay: Some(OverlayAction::Dismiss),
                ..EventResult::default()
            },
            _ => EventResult::consumed(),
        }
    }
}
