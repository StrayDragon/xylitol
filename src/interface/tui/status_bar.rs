//! Status bar component for the TUI.

use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Widget;

/// Status bar — single-line status indicator at the bottom.
pub(crate) struct StatusBar {
    /// Current status message.
    message: String,
    /// Whether the agent is active.
    running: bool,
}

impl StatusBar {
    pub(crate) fn new() -> Self {
        Self {
            message: "Ready. Type a message and press Enter to start.".into(),
            running: false,
        }
    }

    /// Set status message.
    pub(crate) fn set_message(&mut self, msg: impl Into<String>) {
        self.message = msg.into();
    }

    /// Set running state.
    pub(crate) fn set_running(&mut self, running: bool) {
        self.running = running;
    }
}

impl Widget for &StatusBar {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let style = if self.running {
            Style::default().fg(Color::Black).bg(Color::Green)
        } else {
            Style::default().fg(Color::White).bg(Color::Blue)
        };

        let icon = if self.running { " ▶ " } else { " ◼ " };

        // Fill background.
        for x in area.x..area.right() {
            for y in area.y..area.bottom() {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_style(style);
                    cell.set_symbol(" ");
                }
            }
        }

        let text = format!("{}{}", icon, self.message);
        let max_w = area.width as usize;
        let display = if text.len() > max_w {
            format!("{}…", &text[..max_w.saturating_sub(1)])
        } else {
            text
        };

        buf.set_string(area.x, area.y, &display, style);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    #[test]
    fn test_render() {
        let sb = StatusBar::new();
        let mut buf = Buffer::empty(Rect::new(0, 0, 40, 1));
        sb.render(buf.area, &mut buf);
        let cell_bytes = buf
            .cell(ratatui::layout::Position::new(0, 0))
            .unwrap()
            .symbol()
            .as_bytes()
            .to_vec();
        let text = String::from_utf8_lossy(&cell_bytes);
        assert!(!text.is_empty());
    }
}
