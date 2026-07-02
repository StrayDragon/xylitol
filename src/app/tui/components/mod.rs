//! Reusable, individually-testable widgets for the inline TUI tail region.
//!
//! Each widget is self-contained: it consumes only UI-only data types
//! (`RenderedLine` / `&str` / style / input buffer) and renders into a
//! `ratatui_core::buffer::Buffer` via the `Widget` trait — so it is verifiable
//! with a `TestBackend` and composable by the layout widget [`tail::Tail`]. No
//! widget imports agent/infra internals or matches `XyEvent` (spec tui41/tui42).
//! The business→UI seam (`xyevent_to_rendered`) stays in `render.rs`; widgets
//! never see `XyEvent`.

pub mod bottom_panel;
pub mod input_prompt;
pub mod mutable_line;
pub mod tail;
pub mod transcript_line;

pub use tail::Tail;
pub use transcript_line::TranscriptLine;
