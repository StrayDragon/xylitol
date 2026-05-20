//! Transcript overlay — read-only full-screen view of the chat transcript.

use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::interface::tui::component::{Component, EventResult, OverlayAction};
use crate::interface::tui::event::TuiEvent;

pub(crate) struct TranscriptOverlay {
    dirty: bool,
    lines: Vec<Line<'static>>,
    scroll: u16,
}

impl TranscriptOverlay {
    pub(crate) fn set_mode_raw(&mut self, raw: bool) {
        if raw {
            // When we later implement a true raw transcript snapshot, this hook will switch
            // the overlay into that representation. For now, force redraw so toggling raw mode
            // refreshes the overlay content if upstream changes.
            self.dirty = true;
        } else {
            self.dirty = true;
        }
    }
}

impl TranscriptOverlay {
    pub(crate) fn new(markdown: String) -> Self {
        let mut lines = Vec::new();
        for line in markdown.lines() {
            lines.push(Line::from(Span::raw(line.to_string())));
        }
        if lines.is_empty() {
            lines.push(Line::from(Span::raw("(empty transcript)")));
        }

        Self {
            dirty: true,
            lines,
            scroll: 0,
        }
    }

    fn centered(area: Rect, width: u16, height: u16) -> Rect {
        let w = width.min(area.width);
        let h = height.min(area.height);
        let x = area.x + (area.width.saturating_sub(w) / 2);
        let y = area.y + (area.height.saturating_sub(h) / 2);
        Rect::new(x, y, w, h)
    }

    fn scroll_up(&mut self, n: u16) {
        self.scroll = self.scroll.saturating_sub(n);
        self.dirty = true;
    }

    fn scroll_down(&mut self, n: u16) {
        self.scroll = self.scroll.saturating_add(n);
        self.dirty = true;
    }
}

impl Component for TranscriptOverlay {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let modal = Self::centered(area, area.width, area.height);
        frame.render_widget(Clear, modal);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(" Transcript ")
            .title_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );

        let mut header = vec![
            Line::from(vec![
                Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" — close  "),
                Span::styled("j/k", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" or "),
                Span::styled("↑/↓", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" — scroll"),
            ]),
            Line::from(""),
        ];
        header.extend(self.lines.iter().cloned());

        let para = Paragraph::new(Text::from(header))
            .block(block)
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: false })
            .scroll((self.scroll, 0))
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

        use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
        if key.kind != KeyEventKind::Press {
            return EventResult::default();
        }

        if matches!(key.code, KeyCode::Esc | KeyCode::Char('q')) {
            return EventResult {
                consumed: true,
                overlay: Some(OverlayAction::Dismiss),
                ..EventResult::default()
            };
        }

        if key.modifiers.is_empty() {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.scroll_up(1);
                    return EventResult::consumed();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.scroll_down(1);
                    return EventResult::consumed();
                }
                KeyCode::PageUp => {
                    self.scroll_up(10);
                    return EventResult::consumed();
                }
                KeyCode::PageDown => {
                    self.scroll_down(10);
                    return EventResult::consumed();
                }
                _ => {}
            }
        }

        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('u') => {
                    self.scroll_up(10);
                    return EventResult::consumed();
                }
                KeyCode::Char('d') => {
                    self.scroll_down(10);
                    return EventResult::consumed();
                }
                _ => {}
            }
        }

        EventResult::consumed()
    }
}
