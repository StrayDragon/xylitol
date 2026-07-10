//! Expandable multi-line output with a max-height viewport (pi bash/tool preview).
//!
//! Collapsed: show the **last** `max_preview_lines` visual lines (wrap-aware) with a
//! dim hint `... (N earlier lines, …)`. Expanded: full content. Host owns the
//! expand keybinding (pi: `Ctrl+O` / `app.tools.expand`).

use crate::tui::Component;
use crate::utils::{TruncateFrom, VisualTruncateResult, truncate_to_visual_lines};

/// Options for collapsed preview rendering.
#[derive(Debug, Clone)]
pub struct ExpandableOutputOptions {
    /// Max visual lines when collapsed (pi bash tool = 5, bash-mode = 20).
    pub max_preview_lines: usize,
    /// Tail = last-N (logs/bash); Head = first-N (read/grep style).
    pub from: TruncateFrom,
    /// Trailing phrase after the skipped count, e.g. `"ctrl+o to expand"`.
    pub expand_hint: String,
    /// Optional style for the hint line (defaults to dim SGR).
    pub hint_style: Option<fn(&str) -> String>,
}

impl Default for ExpandableOutputOptions {
    fn default() -> Self {
        Self {
            max_preview_lines: 5,
            from: TruncateFrom::Tail,
            expand_hint: "ctrl+o to expand".into(),
            hint_style: None,
        }
    }
}

fn default_dim(s: &str) -> String {
    format!("\x1b[2m{s}\x1b[22m")
}

/// Render preview or full output (pure function for tests / composition).
pub fn render_expandable_output(
    text: &str,
    width: usize,
    expanded: bool,
    opts: &ExpandableOutputOptions,
) -> Vec<String> {
    if expanded || text.is_empty() {
        let VisualTruncateResult { visual_lines, .. } =
            truncate_to_visual_lines(text, usize::MAX, width, TruncateFrom::Tail);
        return visual_lines;
    }

    let max = opts.max_preview_lines.max(1);
    let VisualTruncateResult {
        visual_lines,
        skipped_count,
    } = truncate_to_visual_lines(text, max, width, opts.from);

    if skipped_count == 0 {
        return visual_lines;
    }

    let word = match opts.from {
        TruncateFrom::Tail => "earlier",
        TruncateFrom::Head => "more",
    };
    let hint_raw = format!("... ({skipped_count} {word} lines, {})", opts.expand_hint);
    let style = opts.hint_style.unwrap_or(default_dim);
    let hint = style(&hint_raw);

    let mut out = Vec::with_capacity(visual_lines.len() + 1);
    // pi bash tool: hint above the visible tail (image 1).
    if matches!(opts.from, TruncateFrom::Tail) {
        out.push(hint);
        out.extend(visual_lines);
    } else {
        out.extend(visual_lines);
        out.push(hint);
    }
    out
}

/// Component wrapper: host toggles [`Self::set_expanded`]; content may stream via [`Self::set_text`].
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

    pub fn toggle_expanded(&mut self) {
        self.expanded = !self.expanded;
    }
}

impl Component for ExpandableOutput {
    fn render(&mut self, width: usize) -> Vec<String> {
        render_expandable_output(&self.text, width, self.expanded, &self.options)
    }

    fn handle_input(&mut self, _event: crate::tui::InputEvent) {}

    fn invalidate(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapsed_tail_shows_hint_then_last_lines() {
        let text = (1..=20)
            .map(|i| format!("line-{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let opts = ExpandableOutputOptions {
            max_preview_lines: 3,
            from: TruncateFrom::Tail,
            expand_hint: "ctrl+o to expand".into(),
            hint_style: Some(|s| s.to_string()),
        };
        let lines = render_expandable_output(&text, 40, false, &opts);
        assert!(
            lines[0].contains("17 earlier lines") && lines[0].contains("ctrl+o to expand"),
            "hint first: {lines:?}"
        );
        assert_eq!(lines.len(), 4); // hint + 3
        assert!(lines[3].contains("line-20"), "tail last: {lines:?}");
    }

    #[test]
    fn expanded_shows_all_without_hint() {
        let text = (1..=8)
            .map(|i| format!("L{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let opts = ExpandableOutputOptions {
            max_preview_lines: 3,
            ..ExpandableOutputOptions::default()
        };
        let lines = render_expandable_output(&text, 40, true, &opts);
        assert_eq!(lines.len(), 8);
        assert!(!lines.iter().any(|l| l.contains("earlier")));
    }

    #[test]
    fn streaming_tail_sticks_to_bottom() {
        let mut out = ExpandableOutput::new(
            "",
            ExpandableOutputOptions {
                max_preview_lines: 2,
                from: TruncateFrom::Tail,
                expand_hint: "ctrl+o to expand".into(),
                hint_style: Some(|s| s.to_string()),
            },
        );
        for i in 1..=10 {
            out.append_text(&format!("row{i}\n"));
        }
        let lines = out.render(40);
        assert!(lines[0].contains("earlier"));
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
        // row1 alone would match row10 — check prefix carefully
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
}
