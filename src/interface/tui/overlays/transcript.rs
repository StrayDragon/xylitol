//! Transcript overlay — read-only full-screen view of the chat transcript.

use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::interface::tui::component::{Component, EventResult, OverlayAction};
use crate::interface::tui::event::{AppAction, TuiEvent};

pub(crate) struct TranscriptOverlay {
    dirty: bool,
    raw_lines: Vec<String>,
    scroll: u16,
    cursor: usize,
    selection_start: Option<usize>,
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
        let mut raw_lines = markdown.lines().map(|l| l.to_string()).collect::<Vec<_>>();
        if raw_lines.is_empty() {
            raw_lines.push("(empty transcript)".to_string());
        }

        let cursor = raw_lines.len().saturating_sub(1);

        Self {
            dirty: true,
            raw_lines,
            scroll: 0,
            cursor,
            selection_start: None,
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

    fn clamp_scroll(&mut self, viewport_height: u16) {
        let viewport_height = viewport_height.max(1) as usize;
        let max_scroll = self
            .raw_lines
            .len()
            .saturating_sub(viewport_height)
            .min(u16::MAX as usize) as u16;
        self.scroll = self.scroll.min(max_scroll);
    }

    fn ensure_cursor_visible(&mut self, viewport_height: u16) {
        let viewport_height = viewport_height.max(1) as usize;
        let scroll = self.scroll as usize;
        if self.cursor < scroll {
            self.scroll = self.cursor.min(u16::MAX as usize) as u16;
            return;
        }
        if self.cursor >= scroll.saturating_add(viewport_height) {
            let next_scroll = self
                .cursor
                .saturating_add(1)
                .saturating_sub(viewport_height);
            self.scroll = next_scroll.min(u16::MAX as usize) as u16;
        }
    }

    fn selection_range(&self) -> Option<(usize, usize)> {
        let start = self.selection_start?;
        let a = start.min(self.cursor);
        let b = start.max(self.cursor);
        Some((a, b))
    }

    fn move_cursor(&mut self, delta: i32) {
        if self.raw_lines.is_empty() {
            self.cursor = 0;
            return;
        }
        let len = self.raw_lines.len() as i32;
        let next = (self.cursor as i32 + delta).clamp(0, len.saturating_sub(1));
        self.cursor = next as usize;
        self.dirty = true;
    }

    fn toggle_selection(&mut self) {
        if self.selection_start.is_some() {
            self.selection_start = None;
        } else {
            self.selection_start = Some(self.cursor);
        }
        self.dirty = true;
    }

    fn selected_text(&self) -> Option<String> {
        let (a, b) = self.selection_range()?;
        let mut out = String::new();
        for (idx, line) in self.raw_lines[a..=b].iter().enumerate() {
            if idx > 0 {
                out.push('\n');
            }
            out.push_str(line);
        }
        Some(out)
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

        let inner = block.inner(modal);
        frame.render_widget(block, modal);

        // Header (fixed) + body (scrollable).
        let chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Length(2),
                ratatui::layout::Constraint::Min(0),
            ])
            .split(inner);
        let header_area = chunks[0];
        let body_area = chunks[1];

        let header = vec![
            Line::from(vec![
                Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" close  "),
                Span::styled("j/k", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("/"),
                Span::styled("↑/↓", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" move  "),
                Span::styled("v", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" select  "),
                Span::styled("y/Enter", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" copy"),
            ]),
            Line::from(""),
        ];
        frame.render_widget(
            Paragraph::new(Text::from(header))
                .alignment(Alignment::Left)
                .wrap(Wrap { trim: false })
                .style(Style::default().fg(Color::Reset)),
            header_area,
        );

        // Keep cursor & scroll stable within the visible body height.
        self.clamp_scroll(body_area.height);
        self.ensure_cursor_visible(body_area.height);
        self.clamp_scroll(body_area.height);

        let cursor_style = Style::default().add_modifier(Modifier::REVERSED);
        let selection_style = Style::default().fg(Color::Black).bg(Color::LightCyan);

        let selection = self.selection_range();
        let body_lines = self
            .raw_lines
            .iter()
            .enumerate()
            .map(|(idx, raw)| {
                let mut line = Line::from(Span::raw(raw.clone()));
                if let Some((a, b)) = selection
                    && idx >= a
                    && idx <= b
                {
                    line.style = selection_style;
                } else if idx == self.cursor {
                    line.style = cursor_style;
                }
                line
            })
            .collect::<Vec<_>>();

        let para = Paragraph::new(Text::from(body_lines))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: false })
            .scroll((self.scroll, 0))
            .style(Style::default().fg(Color::Reset));
        frame.render_widget(para, body_area);

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
        if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
            return EventResult::default();
        }

        if matches!(key.code, KeyCode::Esc | KeyCode::Char('q')) {
            return EventResult {
                consumed: true,
                overlay: Some(OverlayAction::Dismiss),
                ..EventResult::default()
            };
        }

        // Copy selected range (or current line) and exit.
        if key.code == KeyCode::Enter
            || (key.code == KeyCode::Char('y') && key.modifiers.is_empty())
        {
            let text = self
                .selected_text()
                .unwrap_or_else(|| self.raw_lines.get(self.cursor).cloned().unwrap_or_default());
            return EventResult {
                consumed: true,
                action: Some(AppAction::CopyText(text)),
                overlay: Some(OverlayAction::Dismiss),
            };
        }

        if key.modifiers.is_empty() {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.move_cursor(-1);
                    return EventResult::consumed();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.move_cursor(1);
                    return EventResult::consumed();
                }
                KeyCode::PageUp => {
                    self.move_cursor(-10);
                    return EventResult::consumed();
                }
                KeyCode::PageDown => {
                    self.move_cursor(10);
                    return EventResult::consumed();
                }
                KeyCode::Char('v') => {
                    self.toggle_selection();
                    return EventResult::consumed();
                }
                _ => {}
            }
        }

        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('u') => {
                    self.move_cursor(-10);
                    return EventResult::consumed();
                }
                KeyCode::Char('d') => {
                    self.move_cursor(10);
                    return EventResult::consumed();
                }
                _ => {}
            }
        }

        EventResult::consumed()
    }
}
