//! Composer rendering to ANSI-styled lines.
//!
//! Converts the pure `Composer` state into `Vec<String>` lines.
//! Layout (from top to bottom in the fixed area):
//!
//! ```text
//! ──────────────────────────────────────────────  ← top separator
//!   <draft text or placeholder>                  ← input area
//! ──────────────────────────────────────────────  ← bottom separator
//!   ~/path (branch) • model | thinking            ← status bar
//! ```
//!
//! Pure characters only — no emoji.

use crate::interface::tui::engine::ansi;
use crate::interface::tui::state::Composer;

/// Render the composer area into ANSI-styled lines.
///
/// Returns lines in this order:
///   0: top separator  (───)
///   1: input line     (cursor marker, draft or placeholder)
///   2: bottom separator (───)
///   3: status bar
///
/// For multi-line drafts, additional input lines are inserted between
/// the top separator and the bottom separator, with the cursor marker
/// on the last draft line.
pub(crate) fn render_composer(
    composer: &Composer,
    max_width: u16,
    is_running: bool,
    status_line: &str,
) -> Vec<String> {
    let mut lines = Vec::new();

    // ── Line 0: Top separator ────────────────────────────────────
    lines.push(ansi::horizontal_rule(max_width));

    // ── Input line(s) ────────────────────────────────────────────
    let draft = composer.draft();
    if draft.is_empty() {
        // Place cursor BEFORE the dimmed placeholder so the hardware cursor
        // lands right where the user will start typing.
        let placeholder_text = "  Type a message...";
        let truncated =
            ansi::truncate_visible(placeholder_text, max_width.saturating_sub(1) as usize);
        let placeholder = ansi::dim_text(&truncated);
        let line = format!("{}{placeholder}", ansi::CURSOR_MARKER);
        lines.push(ansi::pad_to_width(&line, max_width));
    } else {
        let draft_line_count = draft.lines().count();
        for (i, dl) in draft.lines().enumerate() {
            let is_last = i + 1 == draft_line_count;
            let truncated = ansi::truncate_visible(dl, max_width.saturating_sub(2) as usize);
            let content = format!("  {truncated}");
            if is_last {
                lines.push(ansi::pad_to_width(
                    &format!("{content}{}", ansi::CURSOR_MARKER),
                    max_width,
                ));
            } else {
                lines.push(ansi::pad_to_width(&content, max_width));
            }
        }
    }

    // ── Bottom separator ─────────────────────────────────────────
    lines.push(ansi::horizontal_rule(max_width));

    // ── Status bar ───────────────────────────────────────────────
    let status_text = build_status_bar(
        is_running,
        composer.queue_len(),
        status_line,
        composer.bang_shell,
    );
    lines.push(ansi::pad_to_width(&status_text, max_width));

    lines
}

/// Build the status bar text from its components.
fn build_status_bar(
    is_running: bool,
    queue_len: usize,
    status_line: &str,
    bang_shell: bool,
) -> String {
    // Running indicator
    let run_part = if is_running {
        ansi::hint("running")
    } else if queue_len > 0 {
        ansi::accent("idle (queued)")
    } else {
        ansi::dim_text("idle")
    };

    let mut parts: Vec<String> = vec![format!("  {run_part}")];

    // Queue length
    if queue_len > 0 {
        parts.push(format!(" [Q:{queue_len}]"));
    }

    // Bang shell indicator
    if bang_shell {
        parts.push(format!(" {}", ansi::hint("[shell]")));
    }

    // Status line (model, path, etc.)
    if !status_line.is_empty() {
        parts.push(format!(" | {}", ansi::dim_text(status_line)));
    }

    parts.concat()
}

/// StatusInfo for the status bar — richer context.
///
/// This is separate from the renderer so callers can pass in
/// dynamically collected information (cwd, branch, model, etc.).
#[derive(Debug, Clone)]
pub(crate) struct StatusInfo {
    pub(crate) cwd: String,
    pub(crate) branch: String,
    pub(crate) context_progress: String,
    pub(crate) provider: String,
    pub(crate) model_name: String,
    pub(crate) thinking: String,
}

impl StatusInfo {
    pub(crate) fn new() -> Self {
        Self {
            cwd: String::new(),
            branch: String::new(),
            context_progress: String::new(),
            provider: String::new(),
            model_name: String::new(),
            thinking: String::new(),
        }
    }

    /// Format as a compact status line.
    /// Example: ~/path (main) • 0.0%/1.0M • (provider) model | high
    pub(crate) fn format(&self, max_width: u16) -> String {
        let mut parts: Vec<String> = Vec::new();

        // CWD with branch
        let mut cwd_part = self.cwd.clone();
        if !self.branch.is_empty() {
            cwd_part = format!("{cwd_part} ({})", self.branch);
        }
        parts.push(cwd_part);

        // Context progress
        if !self.context_progress.is_empty() {
            parts.push(self.context_progress.clone());
        }

        // Model info
        let mut model_part = String::new();
        if !self.provider.is_empty() {
            model_part = format!("({}) {}", self.provider, self.model_name);
        } else if !self.model_name.is_empty() {
            model_part = self.model_name.clone();
        }
        if !model_part.is_empty() {
            parts.push(model_part);
        }

        // Thinking level
        if !self.thinking.is_empty() {
            // Append to the model part with a pipe
            if let Some(last) = parts.last_mut() {
                last.push_str(&format!(" | {}", self.thinking));
            }
        }

        let joined = parts.join(" • ");
        if ansi::visible_width(&joined) > max_width as usize {
            ansi::truncate_visible(&joined, max_width as usize)
        } else {
            joined
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interface::tui::state::Composer;

    fn visual_lines(composer: &Composer, is_running: bool) -> Vec<String> {
        render_composer(composer, 80, is_running, "test status")
    }

    #[test]
    fn test_empty_composer() {
        let c = Composer::new();
        let lines = visual_lines(&c, false);
        // Top sep + input + bottom sep + status = 4
        assert_eq!(lines.len(), 4, "should have 4 lines");
        assert!(lines[0].contains("─"), "line 0 should be a horizontal rule");
        assert!(
            lines[1].contains("Type a message"),
            "line 1 should have placeholder"
        );
        assert!(lines[2].contains("─"), "line 2 should be a horizontal rule");
        assert!(lines[3].contains("idle"), "line 3 should include idle");
    }

    #[test]
    fn test_empty_cursor_before_placeholder() {
        let c = Composer::new();
        let lines = visual_lines(&c, false);
        let line = &lines[1];
        let marker_pos = line
            .find(ansi::CURSOR_MARKER)
            .expect("cursor marker present on empty draft");
        let placeholder_pos = line.find("Type a message").expect("placeholder present");
        assert!(
            marker_pos < placeholder_pos,
            "cursor marker must come before the placeholder"
        );
    }

    #[test]
    fn test_with_text() {
        let mut c = Composer::new();
        c.set_draft("hello");
        let lines = visual_lines(&c, false);
        assert_eq!(lines.len(), 4);
        assert!(lines[1].contains("hello"));
        assert!(lines[3].contains("idle"));
    }

    #[test]
    fn test_running_state() {
        let c = Composer::new();
        let lines = render_composer(&c, 80, true, "model");
        assert!(lines[3].contains("running"));
    }

    #[test]
    fn test_with_queue() {
        let mut c = Composer::new();
        c.queue.push_back("q1".into());
        let lines = render_composer(&c, 80, false, "model");
        assert!(lines[3].contains("[Q:1]"));
    }

    #[test]
    fn test_bang_shell() {
        let mut c = Composer::new();
        c.set_draft("!echo hi");
        let lines = render_composer(&c, 80, false, "model");
        assert!(lines[3].contains("[shell]"));
    }

    #[test]
    fn test_multi_line_draft() {
        let mut c = Composer::new();
        c.set_draft("line1\nline2\nline3");
        let lines = render_composer(&c, 80, false, "model");
        // top sep + 3 input lines + bottom sep + status = 6
        assert_eq!(lines.len(), 6);
        assert!(lines[1].contains("line1"));
        assert!(lines[2].contains("line2"));
        assert!(lines[3].contains("line3"));
        // cursor on last input line
        assert!(
            lines[3].contains(ansi::CURSOR_MARKER),
            "cursor marker on last draft line"
        );
    }

    #[test]
    fn test_status_line_shown() {
        let c = Composer::new();
        let lines = render_composer(&c, 80, false, "custom-status");
        assert!(lines[3].contains("custom-status"));
    }

    #[test]
    fn test_width_respected() {
        let mut c = Composer::new();
        c.set_draft("a short draft");
        let lines = render_composer(&c, 30, false, "test");
        for line in &lines {
            let vw = ansi::visible_width(line);
            assert!(
                vw <= 30,
                "line width {vw} exceeds 30: {:?}",
                ansi::strip_ansi(line)
            );
        }
    }

    #[test]
    fn test_draft_whitespace_preserved() {
        let mut c = Composer::new();
        c.set_draft("a ");
        let lines = visual_lines(&c, false);
        let visible = ansi::strip_ansi(&lines[1]);
        assert!(
            visible.contains("a "),
            "trailing space must be preserved, got: {visible:?}"
        );
    }

    // ── StatusInfo tests ──

    #[test]
    fn test_status_info_empty() {
        let si = StatusInfo::new();
        let formatted = si.format(80);
        assert!(formatted.is_empty() || formatted == " • " || formatted == "");
        // Actually: all empty → empty string
    }

    #[test]
    fn test_status_info_with_cwd() {
        let mut si = StatusInfo::new();
        si.cwd = "~/project".into();
        let formatted = si.format(80);
        assert!(formatted.contains("~/project"));
    }

    #[test]
    fn test_status_info_with_branch() {
        let mut si = StatusInfo::new();
        si.cwd = "~/project".into();
        si.branch = "main".into();
        let formatted = si.format(80);
        assert!(formatted.contains("(main)"));
    }

    #[test]
    fn test_status_info_full() {
        let mut si = StatusInfo::new();
        si.cwd = "~/project".into();
        si.branch = "main".into();
        si.context_progress = "0.0%/1.0M".into();
        si.provider = "openai".into();
        si.model_name = "gpt-4".into();
        si.thinking = "high".into();
        let formatted = si.format(80);
        assert!(formatted.contains("~/project (main)"));
        assert!(formatted.contains("0.0%/1.0M"));
        assert!(formatted.contains("(openai)"));
        assert!(formatted.contains("gpt-4"));
        assert!(formatted.contains("| high"));
    }

    #[test]
    fn test_status_info_truncated() {
        let mut si = StatusInfo::new();
        si.cwd = "~/a-very-long-path-that-exceeds-width limits-testing".into();
        let formatted = si.format(20);
        assert!(ansi::visible_width(&formatted) <= 20);
    }
}
