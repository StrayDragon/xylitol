use xylitol_tui::truncate_to_width;

use crate::layout::LayoutTheme;

const LANE_LABEL_WIDTH: usize = 9;
const ROW_PREFIX: &str = " ┆";
const HINT_PREFIX: &str = " ↳";
const QUEUE_HINT: &str = "click a row to recall it · click this line to recall all";

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
    let hint = theme.paint_muted(&format!("{HINT_PREFIX} {QUEUE_HINT}"));
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
    for text in messages.iter() {
        let head = format!("{ROW_PREFIX} {label:<LANE_LABEL_WIDTH$}  ");
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
#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::LayoutTheme;
    use xylitol_tui::visible_width;

    fn strip(steer: &[&str], follow: &[&str]) -> Vec<String> {
        let steer: Vec<String> = steer.iter().map(|s| (*s).to_string()).collect();
        let follow: Vec<String> = follow.iter().map(|s| (*s).to_string()).collect();
        render_queue_strip(LayoutTheme::product_dark(), &steer, &follow, 80)
    }

    #[test]
    fn hint_and_rows_share_one_left_edge() {
        let lines = strip(&["a"], &["b"]);
        let plain: Vec<String> = lines
            .iter()
            .map(|l| xylitol_tui::utils::strip_ansi_codes(l))
            .collect();
        let gutter = |line: &str| line.chars().take_while(|c| *c == ' ').count();
        let rows: Vec<&String> = plain.iter().filter(|l| l.contains('┆')).collect();
        let hint = plain.iter().find(|l| l.contains('↳')).expect("hint");
        assert_eq!(rows.len(), 2, "one row per queued message: {plain:?}");
        assert_eq!(
            gutter(hint),
            gutter(rows[0]),
            "the hint must sit on the same column as the row marker: {plain:?}"
        );
        let body_col = |line: &str, marker: char| {
            let at = line.find(marker).expect("marker");
            visible_width(&line[..at])
        };
        let a = body_col(rows[0], '┆');
        let b = body_col(rows[1], '┆');
        assert_eq!(a, b, "lane markers must share a column: {plain:?}");
        let body_a = rows[0].find("a").expect("steer body");
        let body_b = rows[1].find("b").expect("follow-up body");
        assert_ne!(rows[0], rows[1]);
        assert_eq!(
            visible_width(&rows[0][..body_a]),
            visible_width(&rows[1][..body_b]),
            "lane labels are padded so bodies start on one column: {plain:?}"
        );
    }

    #[test]
    fn labels_read_steering_and_follow_up() {
        let plain: Vec<String> = strip(&["a"], &["b"])
            .iter()
            .map(|l| xylitol_tui::utils::strip_ansi_codes(l))
            .collect();
        assert!(plain[1].contains("Steering"), "{plain:?}");
        assert!(plain[2].contains("Follow-up"), "{plain:?}");
    }

    #[test]
    fn hint_uses_click_semantics_without_key_chords() {
        let plain: Vec<String> = strip(&["a"], &[])
            .iter()
            .map(|l| xylitol_tui::utils::strip_ansi_codes(l))
            .collect();
        let hint = plain.iter().find(|l| l.contains('↳')).expect("hint");
        assert!(hint.contains("click"), "{hint}");
        for chord in ["Alt+", "Ctrl+", "Shift+"] {
            assert!(!hint.contains(chord), "hint must not name {chord}: {hint}");
        }
    }
}
