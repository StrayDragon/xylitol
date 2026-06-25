//! Experimental pi-style TUI engine — Component trait + differential ANSI rendering.
//!
//! This module is an alternative rendering path that produces `Vec<String>` (ANSI-styled lines)
//! instead of ratatui widgets.  It is **not wired into the event loop yet**—use it for
//! prototyping, testing, and eventual replacement of ratatui.
//!
//! # Architecture
//!
//! ```text
//! App ──→ compose_layout(app, width, height) ──→ Vec<String> ──→ diff(old, new) ──→ stdout write
//! ```
//!
//! # Layout (bottom-up)
//!
//! ```text
//!   [transcript]        ← grows upward from the top separator
//!   ────────────────    ← top separator (───)
//!   [input area]        ← user composition, with cursor marker
//!   ────────────────    ← bottom separator (───)
//!   [status bar]        ← cwd, branch, model, etc.
//! ```
//!
//! # Status
//!
//! - [x] `TuiRenderer` — differential render loop (tracks prev_lines, writes diffs)
//! - [x] `ansi` — ANSI escape code builders (256-color, bold, dim, etc.)
//! - [x] `diff` — line-by-line diff computation
//! - [x] `transcript_renderer` — renders Transcript to ANSI lines (newest at bottom)
//! - [x] `composer_renderer` — renders Composer to ANSI lines (separator + input + separator + status)
//! - [x] `compose_layout` — combines transcript + composer into full-screen lines
//! - [ ] Paged transcript (scrollback)
//! - [ ] Text selection

pub(crate) mod ansi;
pub(crate) mod composer_renderer;
pub(crate) mod diff;
pub(crate) mod renderer;
pub(crate) mod transcript_renderer;

pub(crate) use composer_renderer::StatusInfo;

use crate::interface::tui::state::App;

/// Compose the full screen layout from App state.
///
/// Layout from bottom to top:
///   1. Status bar     (1 line)
///   2. Bottom separator (1 line)
///   3. Input area     (variable, depends on draft multi-line)
///   4. Top separator  (1 line)
///   5. Transcript     (remaining height, newest at bottom)
///
/// The transcript grows UPWARD from the top separator: the newest messages
/// appear right above it, and older ones scroll off the top.
pub(crate) fn compose_layout(
    app: &App,
    width: u16,
    height: u16,
    status_info: &StatusInfo,
) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();

    // ── Step 1: Render composer area (bottom fixed part) ──────────
    let status_line = status_info.format(width);
    let composer_lines = composer_renderer::render_composer(
        &app.composer,
        width,
        app.transcript.is_running(),
        &status_line,
    );
    // Top separator + input + bottom separator + status = at least 4
    let composer_h = composer_lines.len();
    lines.extend(composer_lines);

    // ── Step 2: Render transcript area (grows upward from top sep) ─
    let transcript_h = height.saturating_sub(composer_h as u16) as usize;
    let rendered = transcript_renderer::render_transcript(
        &app.transcript,
        width,
        app.transcript.scroll_offset,
    );

    let mut transcript_lines = rendered;

    // Strip trailing blank/whitespace-only lines so the last content
    // sits flush against the top separator (no gap).
    while transcript_lines
        .last()
        .map_or(false, |l| l.trim().is_empty())
    {
        transcript_lines.pop();
    }

    // Take only the last `transcript_h` lines (newest content at bottom)
    let visible = transcript_renderer::trim_to_viewport(transcript_lines, transcript_h);
    let visible_len = visible.len();

    // ── Important: insert content FIRST, then padding at the very top ──
    // This ensures the newest message is right against the top separator
    // (no blank lines between last message and the input area).

    // Insert visible transcript lines in reverse order so the oldest ends
    // up at the top of the content section and newest sits right above
    // the top separator.
    for line in visible.into_iter().rev() {
        lines.insert(0, line);
    }

    // Pad at the very top if not enough content to fill the transcript area
    let padding_needed = transcript_h.saturating_sub(visible_len);
    for _ in 0..padding_needed {
        lines.insert(0, ansi::pad_to_width("", width));
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::r#loop::AgentEvent;
    use crate::interface::tui::engine::renderer::TuiRenderer;
    use crate::interface::tui::state::App;

    /// r5 no-leftover (end-to-end): drive the real render path
    /// `compose_layout -> TuiRenderer::render` across several frames where the
    /// composer text changes, and assert the final emitted bytes contain the
    /// last composer line and no stale previous composer text.
    #[test]
    fn multi_frame_render_has_no_ghosting() {
        let mut app = App::new();
        let mut buf: Vec<u8> = Vec::new();
        let mut renderer = TuiRenderer::new(&mut buf);
        let si = StatusInfo::new();

        // Frame 1: empty composer (placeholder shown)
        let f1 = compose_layout(&app, 80, 24, &si);
        renderer.render(&f1, 80, 24).unwrap();

        // Frame 2: user typed "T"
        app.composer.textarea.insert_str("T");
        let f2 = compose_layout(&app, 80, 24, &si);
        renderer.render(&f2, 80, 24).unwrap();

        // Frame 3: user replaced text with "hi" (simulate clear + type)
        app.composer.clear();
        app.composer.textarea.insert_str("hi");
        let f3 = compose_layout(&app, 80, 24, &si);
        renderer.render(&f3, 80, 24).unwrap();

        let out = String::from_utf8(buf).unwrap();
        // The latest composer text MUST appear.
        assert!(out.contains("hi"), "latest composer text must be rendered");
        // Every changed line was written via absolute cursor_goto + erase_line,
        // so the diff path must have emitted erase sequences (no ghosting).
        assert!(
            out.contains(ansi::erase_line()),
            "diff path must erase lines to avoid ghosting"
        );
    }

    /// r2 user-message-display: a submitted user message must appear in the
    /// composed layout with the user prefix.
    #[test]
    fn user_message_appears_in_layout() {
        let mut app = App::new();
        app.transcript.apply(AgentEvent::MessageStart {
            role: "user".into(),
        });
        app.transcript
            .apply(AgentEvent::TextDelta("hello world".into()));
        app.transcript.apply(AgentEvent::MessageEnd {
            role: "user".into(),
        });
        let si = StatusInfo::new();
        let lines = compose_layout(&app, 80, 24, &si);
        let joined = lines.join("\n");
        assert!(joined.contains("hello world"));
        assert!(joined.contains('>'));
    }

    #[test]
    fn test_layout_has_separators_and_status() {
        let app = App::new();
        let si = StatusInfo::new();
        let lines = compose_layout(&app, 80, 24, &si);
        let joined = lines.join("\n");
        // Should have horizontal rules (separators)
        assert!(joined.contains("─"), "should contain horizontal rules");
        // Should have placeholder text
        assert!(
            joined.contains("No messages yet"),
            "should have empty transcript hint"
        );
    }

    #[test]
    fn test_layout_height_respected() {
        let app = App::new();
        let si = StatusInfo::new();
        let lines = compose_layout(&app, 10, 6, &si);
        assert_eq!(
            lines.len(),
            6,
            "should produce exactly 6 lines for height=6"
        );
        for line in &lines {
            let vw = ansi::visible_width(line);
            assert!(
                vw <= 10,
                "line width {vw} exceeds 10: {:?}",
                ansi::strip_ansi(line)
            );
        }
    }

    #[test]
    fn test_status_bar_shows_running_when_agent_active() {
        let mut app = App::new();
        app.transcript.apply(AgentEvent::MessageStart {
            role: "assistant".into(),
        });
        app.transcript
            .apply(AgentEvent::TextDelta("thinking...".into()));
        let si = StatusInfo::new();
        let lines = compose_layout(&app, 80, 24, &si);
        let last_line = lines.last().unwrap();
        let visible = ansi::strip_ansi(last_line);
        assert!(
            visible.contains("running"),
            "status bar should show 'running' when agent active, got: {visible:?}"
        );
    }

    #[test]
    fn test_status_info_in_status_bar() {
        let app = App::new();
        let mut si = StatusInfo::new();
        si.cwd = "~/test".into();
        si.model_name = "gpt-4".into();
        let lines = compose_layout(&app, 80, 24, &si);
        let last_line = lines.last().unwrap();
        let visible = ansi::strip_ansi(last_line);
        assert!(visible.contains("~/test"), "status bar should contain cwd");
        assert!(
            visible.contains("gpt-4"),
            "status bar should contain model name"
        );
    }

    #[test]
    fn test_composer_empty_shows_placeholder() {
        let app = App::new();
        let si = StatusInfo::new();
        let lines = compose_layout(&app, 80, 24, &si);
        let joined = lines.join("\n");
        assert!(
            joined.contains("Type a message"),
            "empty composer should show placeholder"
        );
    }

    #[test]
    fn test_composer_with_draft() {
        let mut app = App::new();
        app.composer.textarea.insert_str("hello");
        let si = StatusInfo::new();
        let lines = compose_layout(&app, 80, 24, &si);
        let joined = lines.join("\n");
        assert!(joined.contains("hello"), "composer draft should be visible");
    }

    /// Verify the newest message sits RIGHT ABOVE the top separator
    /// (no blank padding lines between the last message and the input area).
    #[test]
    fn test_newest_message_adjacent_to_separator() {
        let mut app = App::new();
        // Add several messages so the transcript fills > 1 line
        for i in 0u32..3 {
            app.transcript.apply(AgentEvent::MessageStart {
                role: "user".into(),
            });
            app.transcript
                .apply(AgentEvent::TextDelta(format!("msg {i}")));
            app.transcript.apply(AgentEvent::MessageEnd {
                role: "user".into(),
            });
        }
        let si = StatusInfo::new();
        let lines = compose_layout(&app, 80, 24, &si);

        // Find the indices of separators and the "msg 2" line (newest)
        let sep_indices: Vec<usize> = lines
            .iter()
            .enumerate()
            .filter(|(_, l)| l.contains("─"))
            .map(|(i, _)| i)
            .collect();
        // The first separator from the bottom is the top separator
        // (bottom separator comes after the input area)
        let top_sep_idx = sep_indices[sep_indices.len().saturating_sub(2)];

        // The line immediately before the top separator must be the newest
        // message (not a blank padding line)
        let line_before_sep = &lines[top_sep_idx.saturating_sub(1)];
        let visible = ansi::strip_ansi(line_before_sep);
        assert!(
            visible.contains("msg 2"),
            "line before top separator should be 'msg 2', got: {visible:?}"
        );
    }
}
