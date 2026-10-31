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
    push_queue_line(&mut lines, theme, "Steering", pending_steer, width);
    push_queue_line(&mut lines, theme, "Follow-up", pending_follow_up, width);
    let hint = theme.paint_muted("↳ click to edit all queued messages");
    lines.push(if width == 0 {
        hint
    } else {
        truncate_to_width(&hint, width, "...", true)
    });
    lines
}

fn push_queue_line(
    lines: &mut Vec<String>,
    theme: LayoutTheme,
    label: &str,
    messages: &[String],
    width: usize,
) {
    let Some(latest) = messages.last() else {
        return;
    };
    let label = if messages.len() > 1 {
        format!("{label} ({}): ", messages.len())
    } else {
        format!("{label}: ")
    };
    let line = theme.paint_muted(&format!("{label}{latest}"));
    lines.push(if width == 0 {
        line
    } else {
        truncate_to_width(&line, width, "...", true)
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_queue_renders_no_lines() {
        let lines = render_queue_strip(LayoutTheme::default(), &[], &[], 80);
        assert!(lines.is_empty());
    }

    #[test]
    fn multiple_steer_compacts_to_latest_with_count() {
        let steer = vec!["first".to_string(), "second".to_string()];
        let lines = render_queue_strip(LayoutTheme::default(), &steer, &[], 80);
        let steering: Vec<_> = lines
            .iter()
            .filter(|line| line.contains("Steering"))
            .collect();
        assert_eq!(steering.len(), 1);
        assert!(steering[0].contains("Steering (2): second"));
        assert!(!lines.iter().any(|line| line.contains("first")));
    }

    #[test]
    fn single_steer_has_no_count() {
        let steer = vec!["only".to_string()];
        let lines = render_queue_strip(LayoutTheme::default(), &steer, &[], 80);
        assert!(lines.iter().any(|line| line.contains("Steering: only")));
        assert!(!lines.iter().any(|line| line.contains("Steering (1)")));
    }
}
