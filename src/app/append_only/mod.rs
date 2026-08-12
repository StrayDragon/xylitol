//! Append-only product surface — independent of main-session AO TUI (c2080).
//!
//! See `AGENTS.md`. Specs: `app-tui-append-only` (atao1–atao7).

mod bridge;
mod caps;
mod host;
mod keys;
mod model;
mod spill;

#[cfg(test)]
mod arch_tests;
#[cfg(test)]
mod harness;

pub use caps::{
    APPEND_ONLY_TOOL_BASH_ASSISTANT_LINES, APPEND_ONLY_WRITE_DIFF_LINES, BlockKind,
    MAINLINE_TOOL_PREVIEW_LINES, MAINLINE_WRITE_PREVIEW_LINES,
};
pub use host::{AppendOnlyPreflightError, AppendOnlyRunOptions, AppendOnlySession, preflight, run};
pub use keys::{FORBIDDEN_PRODUCT_ACTIONS, is_allowed_product_action, is_exit_slash};
pub use model::{AppendBlock, AppendOnlyModel};
pub use spill::{
    ensure_session_spill_dir, install_process_spill_dir, process_spill_dir, session_spill_dir,
};

/// Audit note (write-surface step 1 / task 3.0): land zone had no dead append-only
/// skeleton. Existing `#[allow(dead_code)]` in `app/` are reserved remote driver /
/// feature-gated fields — not reused. This module starts clean with real CLI entry.
pub const AUDIT_DEAD_CODE_NOTE: &str =
    "c2080: no dead skeleton in append_only; reserved XyRemoteDriver left untouched";
