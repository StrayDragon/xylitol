//! Composer rendering to ANSI-styled lines.
//!
//! Converts the pure `Composer` state into `Vec<String>` lines,
//! with a prompt marker, draft text, cursor position marker,
//! and a status bar.
//!
//! This replaces `render/composer.rs` (ratatui widgets) for the new engine.

use crate::interface::tui::engine::ansi;
use crate::interface::tui::state::Composer;

/// Total height reserved for the composer area (input + status bar).
pub(crate) const COMPOSER_HEIGHT: u16 = 3;

/// Render the composer area into ANSI-styled lines.
///
/// Returns exactly `COMPOSER_HEIGHT` lines.
/// The cursor marker is placed at the end of the last draft line.
pub(crate) fn render_composer(
    composer: &Composer,
    max_width: u16,
    is_running: bool,
    status_line: &str,
) -> Vec<String> {
    let mut lines = Vec::with_capacity(COMPOSER_HEIGHT as usize);

    // ── Line 1: Prompt + draft text ─────────────────────────────────
    let draft = composer.draft();
    let prompt = if is_running {
        ansi::dim_text("⌛ ")
    } else {
        ansi::user("▸ ")
    };

    if draft.is_empty() {
        // Cursor sits right after the prompt; the dimmed placeholder is
        // rendered AFTER the cursor so typing replaces the visual cue.
        let placeholder = ansi::dim_text("Type a message...");
        let line = format!("  {prompt}{}{placeholder}", ansi::CURSOR_MARKER);
        lines.push(ansi::pad_to_width(&line, max_width));
    } else {
        // Composer is a single-line input area; render the draft verbatim so the
        // user's typed whitespace (leading/trailing spaces, tabs) is preserved.
        // We deliberately do NOT use `ansi::wrap_text` here: that function is a
        // word-wrapper for transcript paragraphs and skips whitespace runs,
        // which would swallow spaces/tabs the user typed one at a time. When the
        // draft exceeds the available width, hard-truncate to the visible width
        // instead of reflowing words.
        // Prefix is "  " + prompt("▸ ") = up to 4 visible columns, plus the
        // CURSOR_MARKER on the last line (zero visible width). Reserve the
        // full 4-column prefix so the truncated content never overflows the row.
        let content_width = max_width.saturating_sub(4) as usize;
        let draft_line_count = draft.lines().count();
        for (i, dl) in draft.lines().enumerate() {
            let is_last = i + 1 == draft_line_count;
            let truncated = ansi::truncate_visible(dl, content_width);
            let wprefix = if i == 0 { &prompt } else { "    " };
            let marker = if is_last { ansi::CURSOR_MARKER } else { "" };
            let line = format!("  {wprefix}{truncated}{marker}");
            lines.push(ansi::pad_to_width(&line, max_width));
        }
    }

    // Fill remaining input lines (if draft has fewer lines than reserved)
    while lines.len() < (COMPOSER_HEIGHT - 1) as usize {
        lines.push(ansi::pad_to_width("", max_width));
    }

    // ── Status bar (last line) ───────────────────────────────────────
    let running_text = if is_running {
        ansi::hint("running")
    } else if composer.has_queue() {
        ansi::accent("idle - queue pending")
    } else {
        ansi::dim_text("idle")
    };

    let mut status_parts = vec![format!(" {}", running_text)];

    if composer.queue_len() > 0 {
        status_parts.push(format!(
            " {}",
            ansi::accent(&format!("[Q:{}]", composer.queue_len()))
        ));
    }

    if !status_line.is_empty() {
        status_parts.push(format!(" | {}", ansi::dim_text(status_line)));
    }

    // For bang shell mode, add a hint
    if composer.bang_shell {
        status_parts.push(format!(" {}", ansi::hint("[shell]")));
    }

    let status_text = status_parts.join("");
    let status_line_padded = ansi::pad_to_width(&status_text, max_width);
    lines.push(status_line_padded);

    lines
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interface::tui::state::Composer;

    fn visual_lines(composer: &Composer, is_running: bool) -> Vec<String> {
        render_composer(composer, 80, is_running, "xylitol TUI")
    }

    #[test]
    fn test_empty_composer() {
        let c = Composer::new();
        let lines = visual_lines(&c, false);
        assert_eq!(lines.len(), COMPOSER_HEIGHT as usize);
        assert!(lines[0].contains("Type a message"));
        assert!(lines.last().unwrap().contains("idle"));
    }

    #[test]
    fn test_empty_composer_cursor_before_placeholder() {
        // r4 empty-cursor: on empty draft the cursor marker MUST precede the
        // placeholder so the hardware cursor lands right after the prompt,
        // not after the dimmed "Type a message..." cue.
        let c = Composer::new();
        let lines = visual_lines(&c, false);
        let line = &lines[0];
        let marker_pos = line
            .find(ansi::CURSOR_MARKER)
            .expect("cursor marker present on empty draft");
        let placeholder_pos = line.find("Type a message").expect("placeholder present");
        assert!(
            marker_pos < placeholder_pos,
            "cursor marker must come before the placeholder, got line: {}",
            ansi::strip_ansi(line)
        );
    }

    /// Regression: typed whitespace (leading/trailing spaces) MUST be rendered
    /// verbatim. The old `wrap_text`-based path skipped whitespace runs, so a
    /// draft like "a " or "  " rendered as if the user had typed nothing.
    #[test]
    fn test_draft_whitespace_preserved() {
        let mut c = Composer::new();
        c.set_draft("a ");
        let lines = visual_lines(&c, false);
        let visible = ansi::strip_ansi(&lines[0]);
        assert!(
            visible.contains("a "),
            "trailing space must be preserved, got: {visible:?}"
        );

        let mut c2 = Composer::new();
        c2.set_draft("  ");
        let lines2 = visual_lines(&c2, false);
        let visible2 = ansi::strip_ansi(&lines2[0]);
        assert!(
            visible2.contains("  "),
            "leading-only spaces must be preserved, got: {visible2:?}"
        );
    }

    #[test]
    fn test_with_text() {
        let mut c = Composer::new();
        c.set_draft("hello");
        let lines = visual_lines(&c, false);
        assert_eq!(lines.len(), COMPOSER_HEIGHT as usize);
        assert!(lines[0].contains("hello"));
        assert!(lines.last().unwrap().contains("idle"));
    }

    #[test]
    fn test_running_state() {
        let c = Composer::new();
        let lines = visual_lines(&c, true);
        assert!(lines.last().unwrap().contains("running"));
    }

    #[test]
    fn test_with_queue() {
        let mut c = Composer::new();
        c.queue.push_back("q1".into());
        let lines = visual_lines(&c, false);
        assert!(lines.last().unwrap().contains("[Q:1]"));
    }

    #[test]
    fn test_bang_shell() {
        let mut c = Composer::new();
        c.set_draft("!echo hi");
        let lines = visual_lines(&c, false);
        assert!(lines.last().unwrap().contains("[shell]"));
    }

    #[test]
    fn test_multi_line_draft() {
        let mut c = Composer::new();
        c.set_draft("line1\nline2\nline3");
        let lines = visual_lines(&c, false);
        assert!(lines[0].contains("line1"));
        assert!(lines[1].contains("line2"));
        assert!(lines[2].contains("line3") || lines[2].starts_with(' '));
        // cursor marker should be on last draft line
        assert!(
            lines.iter().any(|l| l.contains(ansi::CURSOR_MARKER)),
            "cursor marker not found in lines: {lines:?}"
        );
    }

    #[test]
    fn test_status_line_shown() {
        let c = Composer::new();
        let lines = render_composer(&c, 80, false, "custom-status");
        assert!(lines.last().unwrap().contains("custom-status"));
    }

    #[test]
    fn test_width_respected() {
        let mut c = Composer::new();
        c.set_draft("a very long draft text that should be padded properly");
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
}
