//! Steer / follow-up queue strip between scrollback and status (pi morphology).

use xylitol_tui::truncate_to_width;

use crate::app::tui::layout::LayoutTheme;

/// Dim `Steering:` / `Follow-up:` lines + Alt+Up hint. Empty when both queues empty.
pub fn render_queue_strip(
    theme: LayoutTheme,
    pending_steer: &[String],
    pending_follow_up: &[String],
    width: usize,
) -> Vec<String> {
    if pending_steer.is_empty() && pending_follow_up.is_empty() {
        return Vec::new();
    }
    let mut lines = Vec::new();
    // One blank spacer like pi's Spacer(1) before the queue block.
    if width > 0 {
        lines.push(String::new());
    }
    for msg in pending_steer {
        let line = theme.paint_muted(&format!("Steering: {msg}"));
        lines.push(if width == 0 {
            line
        } else {
            truncate_to_width(&line, width, "...", true)
        });
    }
    for msg in pending_follow_up {
        let line = theme.paint_muted(&format!("Follow-up: {msg}"));
        lines.push(if width == 0 {
            line
        } else {
            truncate_to_width(&line, width, "...", true)
        });
    }
    let hint = theme.paint_muted("↳ Alt+Up to edit all queued messages");
    lines.push(if width == 0 {
        hint
    } else {
        truncate_to_width(&hint, width, "...", true)
    });
    lines
}
