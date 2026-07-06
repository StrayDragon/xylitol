//! pi-tui line-array + differential render engine (c399).
//!
//! Self-contained rendering engine modeled on pi-tui's three-module spec
//! (`.agents/skills/tui-pro-of-pi-tui/`): a line-array screen representation
//! diffed per-frame, retained widgets, and a crossterm-backed terminal layer.
//! Replaces the prior ratatui inline-viewport + insert_before model (which
//! forced commit-unit = render-unit = paragraph and lost cross-paragraph
//! markdown context — the c396 screenshot root cause).
//!
//! Build order (see `llmanspec/changes/c399-tui-rewrite-pi-render-engine/tasks.md`):
//! 1. `style` — self-owned CellStyle/Span/StyledLine + ANSI serialization
//! 2. `width` — ANSI-aware width utilities (visible_width/truncate/wrap)
//! 3. `terminal` — crossterm Terminal abstraction + lifecycle
//! 4. `component` — Component/Container/Focusable widget contract
//! 5. `tui` — engine state + doRender pipeline + three diff strategies
//! 6. `virtual_terminal` — in-memory cell-grid test harness

pub mod component;
pub mod style;
pub mod terminal;
pub mod tui;
pub mod virtual_terminal;
pub mod width;
