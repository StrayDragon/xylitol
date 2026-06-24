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
//! # Status
//!
//! - [x] `TuiRenderer` — differential render loop (tracks prev_lines, writes diffs)
//! - [x] `ansi` — ANSI escape code builders (256-color, bold, dim, etc.)
//! - [x] `diff` — line-by-line diff computation
//! - [x] `transcript_renderer` — renders Transcript to ANSI lines
//! - [x] `composer_renderer` — renders Composer to ANSI lines
//! - [x] `compose_layout` — combines transcript + composer into full-screen lines
//! - [ ] Markdown rendering
//! - [ ] Paged transcript (scrollback)
//! - [ ] Text selection

pub(crate) mod ansi;
pub(crate) mod composer_renderer;
pub(crate) mod diff;
pub(crate) mod renderer;
pub(crate) mod transcript_renderer;

use crate::interface::tui::state::App;

/// Compose the full screen layout from App state.
///
/// Returns lines for the entire terminal viewport.
/// - Transcript fills the top portion
/// - Composer (input + status bar) occupies the bottom
pub(crate) fn compose_layout(app: &App, width: u16, height: u16, model_name: &str) -> Vec<String> {
    let composer_h = composer_renderer::COMPOSER_HEIGHT;
    let transcript_h = height.saturating_sub(composer_h);

    let mut lines = Vec::with_capacity(height as usize);

    // Transcript area
    let transcript_lines = transcript_renderer::render_transcript(
        &app.transcript,
        width,
        app.transcript.scroll_offset,
    );
    lines.extend(transcript_lines);

    // Pad transcript to fill remaining space above composer
    while lines.len() < transcript_h as usize {
        lines.push(ansi::pad_to_width("", width));
    }
    // Truncate if transcript is longer than available space
    lines.truncate(transcript_h as usize);

    // Composer area
    let composer_lines = composer_renderer::render_composer(
        &app.composer,
        width,
        app.transcript.is_running(),
        model_name,
    );
    lines.extend(composer_lines);

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
    /// last composer line and no stale previous composer text. This guards the
    /// integration path that produced ghosting in manual testing.
    #[test]
    fn multi_frame_render_has_no_ghosting() {
        let mut app = App::new();
        let mut buf: Vec<u8> = Vec::new();
        let mut renderer = TuiRenderer::new(&mut buf);

        // Frame 1: empty composer (placeholder shown)
        let f1 = compose_layout(&app, 80, 24, "m");
        renderer.render(&f1, 80, 24).unwrap();

        // Frame 2: user typed "T"
        app.composer.textarea.insert_str("T");
        let f2 = compose_layout(&app, 80, 24, "m");
        renderer.render(&f2, 80, 24).unwrap();

        // Frame 3: user replaced text with "hi" (simulate clear + type)
        app.composer.clear();
        app.composer.textarea.insert_str("hi");
        let f3 = compose_layout(&app, 80, 24, "m");
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
        let lines = compose_layout(&app, 80, 24, "m");
        let joined = lines.join("\n");
        assert!(joined.contains("hello world"));
        assert!(joined.contains('▶'));
    }
}
