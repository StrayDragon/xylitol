use crate::tui::Component;
use crate::utils::{TruncateFrom, VisualTruncateResult, truncate_to_visual_lines};

#[derive(Debug, Clone)]
pub struct ExpandableOutputOptions {
    pub max_preview_lines: usize,
    pub from: TruncateFrom,
    pub expand_hint: String,
    pub fold_hint: String,
    pub hint_style: Option<fn(&str) -> String>,
}

impl Default for ExpandableOutputOptions {
    fn default() -> Self {
        Self {
            max_preview_lines: 5,
            from: TruncateFrom::Tail,
            expand_hint: "ctrl+o to expand".into(),
            fold_hint: "ctrl+o to fold".into(),
            hint_style: None,
        }
    }
}

fn default_dim(s: &str) -> String {
    format!("\x1b[2m{s}\x1b[22m")
}

pub fn render_expandable_output(
    text: &str,
    width: usize,
    expanded: bool,
    opts: &ExpandableOutputOptions,
) -> (Vec<String>, Option<usize>) {
    if width == 0 {
        return (Vec::new(), None);
    }
    if expanded || text.is_empty() {
        let VisualTruncateResult { visual_lines, .. } =
            truncate_to_visual_lines(text, usize::MAX, width, TruncateFrom::Tail);
        if expanded && !text.is_empty() && visual_lines.len() > opts.max_preview_lines.max(1) {
            let style = opts.hint_style.unwrap_or(default_dim);
            let mut out = visual_lines;
            out.push(style(&format!("... (expanded, {})", opts.fold_hint)));
            let footer_row = out.len() - 1;
            return (out, Some(footer_row));
        }
        return (visual_lines, None);
    }

    let max = opts.max_preview_lines.max(1);
    let VisualTruncateResult {
        visual_lines,
        skipped_count,
    } = truncate_to_visual_lines(text, max, width, opts.from);

    if skipped_count == 0 {
        return (visual_lines, None);
    }

    let word = match opts.from {
        TruncateFrom::Tail => "earlier",
        TruncateFrom::Head => "more",
    };
    let hint_raw = format!("... ({skipped_count} {word} lines, {})", opts.expand_hint);
    let style = opts.hint_style.unwrap_or(default_dim);
    let hint = style(&hint_raw);

    let mut out = Vec::with_capacity(visual_lines.len() + 1);
    out.extend(visual_lines);
    out.push(hint);
    let footer_row = out.len() - 1;
    (out, Some(footer_row))
}

pub struct ExpandableOutput {
    text: String,
    expanded: bool,
    options: ExpandableOutputOptions,
}

impl ExpandableOutput {
    pub fn new(text: impl Into<String>, options: ExpandableOutputOptions) -> Self {
        Self {
            text: text.into(),
            expanded: false,
            options,
        }
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }

    pub fn append_text(&mut self, chunk: &str) {
        self.text.push_str(chunk);
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn set_expanded(&mut self, expanded: bool) {
        self.expanded = expanded;
    }

    pub fn expanded(&self) -> bool {
        self.expanded
    }
}

impl Component for ExpandableOutput {
    fn render(&mut self, width: usize) -> Vec<String> {
        render_expandable_output(&self.text, width, self.expanded, &self.options).0
    }

    fn handle_input(&mut self, _event: crate::tui::InputEvent) {}

    fn invalidate(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapsed_tail_shows_last_lines_then_hint() {
        let text = (1..=20)
            .map(|i| format!("line-{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let opts = ExpandableOutputOptions {
            max_preview_lines: 3,
            from: TruncateFrom::Tail,
            expand_hint: "ctrl+o to expand".into(),
            fold_hint: "ctrl+o to fold".into(),
            hint_style: Some(|s| s.to_string()),
        };
        let (lines, footer) = render_expandable_output(&text, 40, false, &opts);
        assert_eq!(footer, Some(3), "hint footer row reported: {lines:?}");
        assert!(
            lines[0].contains("line-18"),
            "Tail body starts with window head: {lines:?}"
        );
        assert!(
            !lines[0].contains("earlier lines"),
            "hint must NOT sit above the tail window: {lines:?}"
        );
        assert_eq!(lines.len(), 4);
        assert!(
            lines[2].contains("line-20"),
            "tail window ends with latest: {lines:?}"
        );
        let last = lines.last().expect("hint line");
        assert!(
            last.contains("17 earlier lines") && last.contains("ctrl+o to expand"),
            "Tail hint as block footer: {lines:?}"
        );
    }

    #[test]
    fn expanded_over_limit_adds_fold_footer() {
        let text = (1..=8)
            .map(|i| format!("L{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let opts = ExpandableOutputOptions {
            max_preview_lines: 3,
            ..ExpandableOutputOptions::default()
        };
        let (lines, footer) = render_expandable_output(&text, 40, true, &opts);
        assert_eq!(footer, Some(8), "fold footer row reported: {lines:?}");
        assert_eq!(lines.len(), 9);
        assert!(!lines.iter().any(|l| l.contains("earlier")));
        let last = lines.last().expect("fold footer");
        assert!(
            last.contains("(expanded, ctrl+o to fold)"),
            "expanded fold footer as block footer: {lines:?}"
        );
    }

    #[test]
    fn expanded_within_limit_has_no_footer() {
        let text = "L1\nL2";
        let opts = ExpandableOutputOptions {
            max_preview_lines: 3,
            ..ExpandableOutputOptions::default()
        };
        let (lines, footer) = render_expandable_output(text, 40, true, &opts);
        assert_eq!(footer, None, "short content has no footer: {lines:?}");
        assert_eq!(lines.len(), 2, "short content stays footer-free: {lines:?}");
        assert!(
            !lines.iter().any(|l| l.contains("expanded")),
            "no fold footer below viewport: {lines:?}"
        );
    }

    #[test]
    fn streaming_tail_sticks_to_bottom() {
        let mut out = ExpandableOutput::new(
            "",
            ExpandableOutputOptions {
                max_preview_lines: 2,
                from: TruncateFrom::Tail,
                expand_hint: "ctrl+o to expand".into(),
                fold_hint: "ctrl+o to fold".into(),
                hint_style: Some(|s| s.to_string()),
            },
        );
        for i in 1..=10 {
            out.append_text(&format!("row{i}\n"));
        }
        let lines = out.render(40);
        assert!(
            lines.last().is_some_and(|l| l.contains("earlier")),
            "hint footer: {lines:?}"
        );
        assert!(
            lines.iter().any(|l| l.contains("row10")),
            "tail includes latest row: {lines:?}"
        );
        assert!(
            !lines
                .iter()
                .any(|l| l.contains("row1\n") || l == "row1" || l.ends_with("row1")),
            "early rows hidden: {lines:?}"
        );
        assert!(
            !lines
                .iter()
                .any(|l| l.contains("row1") && !l.contains("row10")),
            "row1 hidden: {lines:?}"
        );
        out.set_expanded(true);
        let full = out.render(40);
        assert!(full.len() >= 10);
        assert!(!full[0].contains("earlier"));
    }

    #[test]
    fn collapsed_head_shows_first_lines_then_more_hint() {
        let text = (1..=20)
            .map(|i| format!("line-{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let opts = ExpandableOutputOptions {
            max_preview_lines: 3,
            from: TruncateFrom::Head,
            expand_hint: "ctrl+o to expand".into(),
            fold_hint: "ctrl+o to fold".into(),
            hint_style: Some(|s| s.to_string()),
        };
        let (lines, footer) = render_expandable_output(&text, 40, false, &opts);
        assert_eq!(footer, Some(3), "more-hint footer row reported: {lines:?}");
        assert!(
            lines[0].contains("line-1"),
            "Head must keep original first line on top: {lines:?}"
        );
        assert!(
            !lines[0].contains("more lines"),
            "more-hint must NOT sit above the head: {lines:?}"
        );
        let last = lines.last().expect("hint line");
        assert!(
            last.contains("17 more lines") && last.contains("ctrl+o to expand"),
            "Head hint below body: {lines:?}"
        );
        assert_eq!(lines.len(), 4);
        assert!(
            lines.iter().any(|l| l.contains("line-3")),
            "preview includes head window: {lines:?}"
        );
        assert!(
            !lines.iter().any(|l| l.contains("line-20")),
            "tail of file hidden in Head collapse: {lines:?}"
        );
    }

    #[test]
    fn empty_text_and_narrow_width_do_not_panic() {
        let opts = ExpandableOutputOptions {
            max_preview_lines: 3,
            from: TruncateFrom::Head,
            expand_hint: "ctrl+o to expand".into(),
            fold_hint: "ctrl+o to fold".into(),
            hint_style: Some(|s| s.to_string()),
        };
        let (empty, _) = render_expandable_output("", 40, false, &opts);
        assert!(empty.len() <= 1, "empty text must be bounded: {empty:?}");

        let text = (1..=12)
            .map(|i| format!("row-{i}-abcdefghijklmnopqrstuvwxyz"))
            .collect::<Vec<_>>()
            .join("\n");
        for w in [0usize, 1] {
            let (collapsed, _) = render_expandable_output(&text, w, false, &opts);
            let (expanded, _) = render_expandable_output(&text, w, true, &opts);
            if w == 0 {
                assert!(
                    collapsed.is_empty(),
                    "width=0 collapsed → empty: {collapsed:?}"
                );
                assert!(
                    expanded.is_empty(),
                    "width=0 expanded → empty: {expanded:?}"
                );
            } else {
                assert!(
                    collapsed.len() < 512,
                    "width=1 collapsed must be bounded: len={}",
                    collapsed.len()
                );
                assert!(
                    expanded.len() < 512,
                    "width=1 expanded must be bounded: len={}",
                    expanded.len()
                );
            }
        }

        let mut comp = ExpandableOutput::new(text, opts);
        let _ = comp.render(0);
        let _ = comp.render(1);
        comp.set_text("");
        assert!(comp.render(0).len() <= 1);
    }
}
