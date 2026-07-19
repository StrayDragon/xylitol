//! Product slash command SSOT re-export for app surfaces (c1175 / seam).
//!
//! Lives under `app/core` so `app/tui` can use the catalog without importing
//! `crate::agent` internals (see `src/AGENTS.md` / write-surface).

pub use crate::agent::prompt::product_commands::product_slash_commands;

#[cfg(any(test, doctest))]
pub use crate::agent::prompt::product_commands::LEGACY_SHORT_NAMES;
