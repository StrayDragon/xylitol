//! Loader widget — animated spinner (c399 stage 2.4).
//!
//! Port of pi's `components/loader.ts`. Shows a single line: a braille-dots
//! spinner frame + a status message (default "Loading..."). The spinner cycles
//! through [`SPINNER`] (10 frames), advanced by the host calling [`advance`].
//!
//! ## Animation model (pi `setInterval` → host tick)
//!
//! pi's Loader holds a `NodeJS.Timeout` and calls `ui.requestRender()` every
//! 80ms — the one sanctioned exception to "widgets don't schedule renders"
//! (ui-components.md Step 5). c399 does not yet have a render scheduler on the
//! engine, and the host loop already runs a steady timer for the legacy
//! spinner (`mod.rs::Msg::Tick`, drives `TuiApp::spinner_idx`). So this widget
//! exposes [`advance`] and stays timer-free; the host calls `advance()` on its
//! tick and requests a render. Same cadence, no `setInterval` equivalent, and
//! the widget remains pure (no I/O, no async).
//!
//! ## Styling
//!
//! Spinner glyph is colored `Color::Cyan` (matches `theme::Palette::spinner`,
//! the c396 asset). The c396 ratatui `Style` → `CellStyle` migration happens in
//! stage 4; for now the color is hard-coded here, like the Markdown widget's
//! syntect adapter (stage 2.2).

use crate::app::tui::engine::component::Component;
use crate::app::tui::engine::style::{Color, Span, StyledLine};
use crate::app::tui::engine::width::truncate;

/// Braille-dots spinner frame cycle (10 frames). Shared definition with the
/// legacy `components::spinner::SPINNER` (c380) and pi's `DEFAULT_FRAMES` —
/// kept here so the new widget layer is self-contained after stage 4 deletes
/// `components/`.
pub const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Default frame interval (pi `DEFAULT_INTERVAL_MS`). The host's tick cadence
/// is the actual driver; this constant is informational for stage-4 wiring
/// (the `Msg::Tick` timer in `mod.rs` already runs at a comparable rate).
pub const DEFAULT_INTERVAL_MS: u64 = 80;

/// Spinner color (c396 `theme::Palette::spinner` = cyan; migrated to CellStyle
/// in stage 4). `pub` so a future theme refactor can override per-instance.
pub const SPINNER_COLOR: Color = Color::Cyan;

/// Animated spinner + status line. Renders exactly one row: `<frame> <message>`,
/// truncated to `width` if the message is long. Call [`Loader::advance`] on each
/// host tick to cycle the frame.
pub struct Loader {
    frames: &'static [&'static str],
    current: usize,
    message: String,
}

impl Default for Loader {
    fn default() -> Self {
        Self::new("Loading...")
    }
}

impl Loader {
    /// New loader with a message; uses the default [`SPINNER`] frames.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            frames: SPINNER,
            current: 0,
            message: message.into(),
        }
    }

    /// Advance the spinner by one frame (wrap around). The host calls this on
    /// its tick — there is no internal timer (see module docs).
    pub fn advance(&mut self) {
        if self.frames.len() > 1 {
            self.current = (self.current + 1) % self.frames.len();
        }
    }

    /// Replace the status message.
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }

    /// Current frame glyph.
    pub fn frame(&self) -> &str {
        self.frames.get(self.current).copied().unwrap_or("")
    }

    /// Current frame index (host may read it for diagnostics / parity checks).
    pub fn current_frame(&self) -> usize {
        self.current
    }

    /// Build the one-line content: spinner span (styled) + " " + message span.
    fn build_line(&self, width: usize) -> StyledLine {
        let frame = self.frame();
        let full = format!("{frame} {}", self.message);
        let line = StyledLine::from_spans(vec![
            Span::styled(
                frame,
                crate::app::tui::engine::style::CellStyle::default().fg(SPINNER_COLOR),
            ),
            Span::raw(&full[frame.len()..]),
        ]);
        truncate(&line, width, "…")
    }
}

impl Component for Loader {
    fn render(&self, width: usize) -> Vec<StyledLine> {
        // pi Loader prepends an empty line (its `render` returns `["", ...super.render()]`)
        // to vertically separate it from content above. The c399 Container lays
        // out children vertically; callers add a Spacer when they want that gap,
        // so this widget emits exactly its content row.
        vec![self.build_line(width)]
    }

    fn invalidate(&mut self) {
        // No cache — render is computed fresh each frame (frame index is
        // external state; cheap to recompute).
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_message_is_loading() {
        let l = Loader::default();
        assert_eq!(l.message, "Loading...");
        assert_eq!(l.frame(), SPINNER[0]);
    }

    #[test]
    fn custom_message() {
        let l = Loader::new("Thinking...");
        assert_eq!(l.message, "Thinking...");
    }

    #[test]
    fn set_message_updates() {
        let mut l = Loader::default();
        l.set_message("done");
        assert_eq!(l.message, "done");
    }

    #[test]
    fn advance_cycles_through_all_frames() {
        let mut l = Loader::default();
        assert_eq!(l.current_frame(), 0);
        // advance len-1 times and confirm each frame lands in order
        for i in 0..SPINNER.len() {
            assert_eq!(l.current_frame(), i, "frame {i}");
            assert_eq!(l.frame(), SPINNER[i]);
            l.advance();
        }
        // wrapped back to 0
        assert_eq!(l.current_frame(), 0);
    }

    #[test]
    fn advance_is_stable_across_many_cycles() {
        let mut l = Loader::default();
        for n in 0..(SPINNER.len() * 5) {
            assert_eq!(l.current_frame(), n % SPINNER.len());
            l.advance();
        }
    }

    #[test]
    fn render_shows_frame_and_message() {
        let l = Loader::new("Working");
        let line = &l.render(40)[0];
        let text = line.plain_text();
        assert!(
            text.starts_with(&format!("{} Working", SPINNER[0])),
            "got {text:?}"
        );
    }

    #[test]
    fn render_advance_changes_frame() {
        let mut l = Loader::new("x");
        let f0 = l.render(40)[0].plain_text();
        l.advance();
        let f1 = l.render(40)[0].plain_text();
        assert_ne!(f0, f1, "frame advanced");
        assert!(f1.starts_with(SPINNER[1]));
    }

    #[test]
    fn render_truncates_long_message_to_width() {
        let l = Loader::new("abcdefghij0123456789abcdefghij0123456789");
        let line = &l.render(10)[0];
        assert!(line.width() <= 10, "width invariant: got {}", line.width());
        // truncated text keeps the spinner prefix + ellipsis marker
        let text = line.plain_text();
        assert!(text.starts_with(SPINNER[0]), "spinner kept: {text:?}");
    }

    #[test]
    fn render_width_invariant_across_widths() {
        let l = Loader::new("a message that is somewhat long");
        for w in [1usize, 3, 5, 10, 40, 80] {
            let line = &l.render(w)[0];
            assert!(line.width() <= w, "width {w} -> {}", line.width());
        }
    }

    #[test]
    fn render_spinner_is_styled() {
        let l = Loader::default();
        let line = &l.render(40)[0];
        // First span is the spinner; it should carry the cyan fg.
        assert!(!line.spans.is_empty());
        assert_eq!(line.spans[0].style.fg, Some(SPINNER_COLOR));
    }

    #[test]
    fn render_one_line_only() {
        let l = Loader::default();
        assert_eq!(l.render(40).len(), 1);
    }

    #[test]
    fn invalidate_is_noop_render_still_works() {
        let mut l = Loader::default();
        l.invalidate();
        assert_eq!(l.render(40).len(), 1);
    }

    #[test]
    fn spinner_const_matches_legacy_and_pi() {
        // Sanity: must equal the c380 legacy const and pi's DEFAULT_FRAMES so
        // animation looks identical before/after stage-4 cutover.
        assert_eq!(SPINNER, &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]);
        assert_eq!(SPINNER.len(), 10);
    }

    #[test]
    fn default_interval_ms_is_80() {
        assert_eq!(DEFAULT_INTERVAL_MS, 80);
    }
}
