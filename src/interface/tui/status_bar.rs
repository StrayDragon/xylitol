//! Status bar component.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::component::{Component, EventResult};
use super::event::TuiEvent;

pub(crate) struct StatusBar {
    running: bool,
    model: String,
    session: String,
    tokens_in: u64,
    tokens_out: u64,
    queued: usize,
    message: String,
    dirty: bool,
}

impl StatusBar {
    pub(crate) fn new() -> Self {
        Self {
            running: false,
            model: "unknown".into(),
            session: "default".into(),
            tokens_in: 0,
            tokens_out: 0,
            queued: 0,
            message: String::new(),
            dirty: true,
        }
    }

    pub(crate) fn set_running(&mut self, running: bool) {
        if self.running != running {
            self.running = running;
            self.dirty = true;
        }
    }

    pub(crate) fn set_model(&mut self, model: impl Into<String>) {
        let model = model.into();
        if self.model != model {
            self.model = model;
            self.dirty = true;
        }
    }

    pub(crate) fn set_session(&mut self, session: impl Into<String>) {
        let session = session.into();
        if self.session != session {
            self.session = session;
            self.dirty = true;
        }
    }

    pub(crate) fn set_token_usage(&mut self, tokens_in: u64, tokens_out: u64) {
        if self.tokens_in != tokens_in || self.tokens_out != tokens_out {
            self.tokens_in = tokens_in;
            self.tokens_out = tokens_out;
            self.dirty = true;
        }
    }

    pub(crate) fn set_queue_len(&mut self, queued: usize) {
        if self.queued != queued {
            self.queued = queued;
            self.dirty = true;
        }
    }

    pub(crate) fn set_message(&mut self, message: impl Into<String>) {
        let message = message.into();
        if self.message != message {
            self.message = message;
            self.dirty = true;
        }
    }
}

impl Component for StatusBar {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let state_style = if self.running {
            Style::default()
                .fg(Color::Black)
                .bg(Color::LightGreen)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(Color::Black)
                .bg(Color::LightBlue)
                .add_modifier(Modifier::BOLD)
        };

        let state = if self.running { " RUNNING " } else { " READY " };
        let mut spans = vec![Span::styled(state, state_style), Span::raw(" ")];

        spans.push(Span::styled(
            format!("model: {}", self.model),
            Style::default().fg(Color::Gray),
        ));
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!("tokens: {}/{}", self.tokens_in, self.tokens_out),
            Style::default().fg(Color::Gray),
        ));
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!("session: {}", self.session),
            Style::default().fg(Color::Gray),
        ));

        if self.queued > 0 {
            spans.push(Span::raw("  "));
            spans.push(Span::styled(
                format!("[Q: {}]", self.queued),
                Style::default().fg(Color::LightYellow),
            ));
        }

        if !self.message.is_empty() {
            spans.push(Span::raw("  "));
            spans.push(Span::styled(
                self.message.clone(),
                Style::default().fg(Color::White),
            ));
        }

        frame.render_widget(Paragraph::new(Line::from(spans)), area);
        self.dirty = false;
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn mark_clean(&mut self) {
        self.dirty = false;
    }

    fn handle_event(&mut self, _event: &TuiEvent) -> EventResult {
        EventResult::default()
    }
}
