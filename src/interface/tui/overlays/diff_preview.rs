//! Diff preview overlay component (modal).

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::interface::diff_review::types::{DiffHunk, DiffLineKind};

use super::super::component::{Component, EventResult, OverlayAction};
use super::super::event::TuiEvent;

pub(crate) struct DiffPreviewOverlay {
    hunks: Vec<DiffHunk>,
    scroll: u16,
    dirty: bool,
}

impl DiffPreviewOverlay {
    pub(crate) fn new(hunks: Vec<DiffHunk>) -> Self {
        Self {
            hunks,
            scroll: 0,
            dirty: true,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn set_diff(&mut self, hunks: Vec<DiffHunk>) {
        self.hunks = hunks;
        self.scroll = 0;
        self.dirty = true;
    }

    fn render_lines(&self, width: u16) -> Vec<Line<'static>> {
        let mut out = Vec::new();

        for hunk in &self.hunks {
            out.push(Line::from(Span::styled(
                format!(
                    "@@ -{},{} +{},{} @@ {}",
                    hunk.old_start, hunk.old_count, hunk.new_start, hunk.new_count, hunk.file
                ),
                Style::default()
                    .fg(Color::LightMagenta)
                    .add_modifier(Modifier::BOLD),
            )));

            for line in &hunk.lines {
                let prefix = line.kind.prefix();
                let content = line.content.trim_end_matches('\n');
                let (fg, bg) = match line.kind {
                    DiffLineKind::Add => (Color::Green, Color::Rgb(10, 40, 10)),
                    DiffLineKind::Delete => (Color::Red, Color::Rgb(40, 10, 10)),
                    DiffLineKind::Context => (Color::Gray, Color::Reset),
                };

                let mut text = String::new();
                text.push_str(prefix);
                text.push_str(content);

                // Truncate to width so the overlay remains readable.
                if width > 0 && text.chars().count() as u16 > width {
                    text = text.chars().take(width as usize).collect();
                }

                let style = if bg == Color::Reset {
                    Style::default().fg(fg)
                } else {
                    Style::default().fg(fg).bg(bg)
                };
                out.push(Line::from(Span::styled(text, style)));
            }

            out.push(Line::from(""));
        }

        if out.is_empty() {
            out.push(Line::from(Span::styled(
                "(no diffs)",
                Style::default().fg(Color::DarkGray),
            )));
        }

        out
    }
}

impl Component for DiffPreviewOverlay {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        // Overlay area — centered, 85% width and height.
        let overlay_w = ((area.width as f32) * 0.85) as u16;
        let overlay_h = ((area.height as f32) * 0.85) as u16;
        let x = area
            .x
            .saturating_add((area.width.saturating_sub(overlay_w)) / 2);
        let y = area
            .y
            .saturating_add((area.height.saturating_sub(overlay_h)) / 2);
        let overlay_rect = Rect::new(x, y, overlay_w.max(50), overlay_h.max(12));

        frame.render_widget(Clear, overlay_rect);

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Diff Preview  (Ctrl+R to close) ")
            .border_style(Style::default().fg(Color::LightMagenta))
            .style(Style::default().bg(Color::Rgb(20, 20, 30)));
        let inner = block.inner(overlay_rect);
        frame.render_widget(block, overlay_rect);

        let view_h = inner.height.saturating_sub(1) as usize;
        let lines = self.render_lines(inner.width.saturating_sub(1));

        let total = lines.len() as u16;
        let max_scroll = total.saturating_sub(view_h as u16);
        let scroll = self.scroll.min(max_scroll);
        let start = scroll as usize;
        let end = (start + view_h).min(lines.len());

        let visible = if start < end {
            lines[start..end].to_vec()
        } else {
            Vec::new()
        };

        let para = Paragraph::new(visible).wrap(Wrap { trim: false });
        frame.render_widget(para, inner);

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

        use crossterm::event::{KeyCode, KeyModifiers};

        match key.code {
            KeyCode::Esc => EventResult {
                consumed: true,
                action: None,
                overlay: Some(OverlayAction::Dismiss),
            },
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => EventResult {
                consumed: true,
                action: None,
                overlay: Some(OverlayAction::Dismiss),
            },
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll = self.scroll.saturating_sub(1);
                self.dirty = true;
                EventResult::consumed()
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll = self.scroll.saturating_add(1);
                self.dirty = true;
                EventResult::consumed()
            }
            KeyCode::Char('g') => {
                self.scroll = 0;
                self.dirty = true;
                EventResult::consumed()
            }
            KeyCode::Char('G') => {
                self.scroll = u16::MAX;
                self.dirty = true;
                EventResult::consumed()
            }
            _ => EventResult::default(),
        }
    }
}
