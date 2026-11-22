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
    push_lane(&mut lines, theme, "Steering", pending_steer, width);
    push_lane(&mut lines, theme, "Follow-up", pending_follow_up, width);
    let hint = theme.paint_muted("↳ click a row to recall it · Alt+Up recalls all");
    lines.push(if width == 0 {
        hint
    } else {
        truncate_to_width(&hint, width, "...", true)
    });
    lines
}

fn push_lane(
    lines: &mut Vec<String>,
    theme: LayoutTheme,
    label: &str,
    messages: &[String],
    width: usize,
) {
    let total = messages.len();
    for (index, text) in messages.iter().enumerate() {
        let head = format!(" ┆ {label} {}/{}  ", index + 1, total);
        let body = truncate_to_width(
            text,
            width
                .saturating_sub(xylitol_tui::visible_width(&head))
                .max(1),
            "…",
            false,
        );
        let line = format!("{}{}", theme.paint_muted(&head), body);
        lines.push(if width == 0 {
            line
        } else {
            truncate_to_width(&line, width, "…", true)
        });
    }
}
