use xylitol_tui::truncate_to_width;

use crate::layout::LayoutTheme;

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
