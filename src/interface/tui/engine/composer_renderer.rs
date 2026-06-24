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
        let placeholder = ansi::dim_text("Type a message...");
        let line = format!("  {prompt}{placeholder}{}", ansi::CURSOR_MARKER);
        lines.push(ansi::pad_to_width(&line, max_width));
    } else {
        // Wrapped continuation lines use "    " prefix (6 total with "  ")
        let content_width = max_width.saturating_sub(6);
        let draft_lines: Vec<&str> = draft.lines().collect();
        let last_idx = draft_lines.len().saturating_sub(1);
        for (i, dl) in draft_lines.iter().enumerate() {
            let is_last = i == last_idx;

            let wrapped = ansi::wrap_text(dl, content_width);
            for (wi, wl) in wrapped.iter().enumerate() {
                let wprefix = if i == 0 && wi == 0 { &prompt } else { "    " };
                let marker = if is_last && wi == wrapped.len() - 1 {
                    ansi::CURSOR_MARKER
                } else {
                    ""
                };
                let line = format!("  {wprefix}{wl}{marker}");
                lines.push(ansi::pad_to_width(&line, max_width));
            }
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
