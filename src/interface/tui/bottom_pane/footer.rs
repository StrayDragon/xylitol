//! Codex-style footer/statusline rendering.
//!
//! This replaces the legacy StatusBar panel with a single-line (or minimal multi-line)
//! hint/status surface, similar to codex-tui.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FooterMode {
    ComposerEmpty,
    ComposerHasDraft,
    EscHint,
    ShortcutOverlay,
}

#[derive(Debug, Clone)]
pub(crate) struct FooterState {
    pub(crate) mode: FooterMode,
    pub(crate) is_task_running: bool,
    pub(crate) queue_len: usize,
    pub(crate) use_shift_enter_hint: bool,
    pub(crate) model_label: String,
    pub(crate) session_label: String,
    pub(crate) message: String,
    message_until: Option<Instant>,
}

impl FooterState {
    pub(crate) fn new() -> Self {
        Self {
            mode: FooterMode::ComposerEmpty,
            is_task_running: false,
            queue_len: 0,
            use_shift_enter_hint: false,
            model_label: "unknown".to_string(),
            session_label: "default".to_string(),
            message: String::new(),
            message_until: None,
        }
    }

    pub(crate) fn set_message(&mut self, message: impl Into<String>, ttl: Duration) {
        self.message = message.into();
        self.message_until = Some(Instant::now() + ttl);
    }

    pub(crate) fn set_sticky_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
        self.message_until = None;
    }

    pub(crate) fn tick(&mut self) {
        let Some(until) = self.message_until else {
            return;
        };
        if Instant::now() >= until {
            self.message.clear();
            self.message_until = None;
        }
    }

    pub(crate) fn render(&self, area: Rect, buf: &mut Buffer) {
        // Codex style: keep it minimal and rely on default terminal palette.
        // Left side is an instruction; right side is context (model/session).
        let left: Span<'static> = if !self.message.trim().is_empty() {
            self.message.clone().cyan()
        } else {
            match self.mode {
                FooterMode::EscHint => "Esc again to edit last message".cyan(),
                FooterMode::ShortcutOverlay => {
                    // Minimal codex-style shortcut overlay as a multi-line footer isn't currently
                    // supported by this simplified FooterState renderer. Keep it single-line and
                    // show the most important keys.
                    "Esc edit previous · Tab queue · Ctrl+T transcript · Ctrl+O copy · Alt+R raw · Alt+Y view thinking"
                        .dim()
                }
                FooterMode::ComposerEmpty => {
                    if self.is_task_running {
                        if self.queue_len > 0 {
                            format!("Running · Tab to queue (Q:{})", self.queue_len).cyan()
                        } else {
                            "Running · Tab to queue".cyan()
                        }
                    } else {
                        "? for shortcuts · Esc to edit previous".dim()
                    }
                }
                FooterMode::ComposerHasDraft => {
                    if self.is_task_running {
                        if self.queue_len > 0 {
                            format!("Tab to queue (Q:{})", self.queue_len).cyan()
                        } else {
                            "Tab to queue".cyan()
                        }
                    } else {
                        if self.use_shift_enter_hint {
                            "Enter to send · Shift+Enter newline".dim()
                        } else {
                            "Enter to send · Ctrl+J newline".dim()
                        }
                    }
                }
            }
        };

        let mut right_text = String::new();
        if !self.model_label.is_empty() {
            right_text.push_str(&self.model_label);
        }
        if !self.session_label.is_empty() {
            if !right_text.is_empty() {
                right_text.push_str(" · ");
            }
            right_text.push_str(&self.session_label);
        }
        let right = right_text.dim();

        let line = compose_left_right(area.width, left, right);
        Paragraph::new(Line::from(line)).render(area, buf);
    }
}

impl FooterState {
    pub(crate) fn toggle_shortcuts_overlay(&mut self) {
        self.mode = match self.mode {
            FooterMode::ShortcutOverlay => FooterMode::ComposerEmpty,
            _ => FooterMode::ShortcutOverlay,
        };
    }
}

fn compose_left_right(width: u16, left: Span<'static>, right: Span<'static>) -> Vec<Span<'static>> {
    let width = width as usize;
    let left_len = left.content.chars().count();
    let right_len = right.content.chars().count();
    if width == 0 {
        return Vec::new();
    }

    // If both fit, pad with spaces.
    if left_len + 1 + right_len <= width {
        let gap = width.saturating_sub(left_len + right_len);
        return vec![left, " ".repeat(gap).into(), right];
    }

    // Prefer keeping the left hint; drop the right side if too tight.
    vec![left]
}
