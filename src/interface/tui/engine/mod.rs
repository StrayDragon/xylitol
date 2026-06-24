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
