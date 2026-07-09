//! Terminal UI application surface (placeholder).
//!
//! The previous in-tree engine/widgets host has been removed. This surface will
//! be redesigned from scratch on top of [`xylitol_tui`] (`packages/xylitol-tui`).
//! See `AGENTS.md` in this directory and `packages/xylitol-tui/AGENTS.md`.
//!
//! Gated behind the `tui` Cargo feature. Until the rewrite lands, [`run`]
//! returns an explicit error so CLI dispatch fails loudly instead of linking
//! dead code.

use crate::app::core::driver::Driver;

/// Enter the interactive TUI REPL.
///
/// **Placeholder:** not implemented. The product TUI will host-drive
/// `xylitol_tui` (async App Shell + sync engine); do not restore the deleted
/// in-tree engine.
pub async fn run(_driver: &mut dyn Driver) -> Result<(), String> {
    Err(
        "TUI is being redesigned on packages/xylitol-tui; the previous \
         src/app/tui implementation was removed. See src/app/tui/AGENTS.md."
            .into(),
    )
}
