//! Product slash command SSOT re-export for app surfaces (c1175 / arch_guard).
//!
//! Lives under `app/core` so `app/tui` can use the catalog without importing
//! `crate::agent` directly (see `tests::arch_guard::app_only_from_driver`).

pub use crate::agent::prompt::product_commands::product_slash_commands;

#[cfg(any(test, doctest))]
pub use crate::agent::prompt::product_commands::LEGACY_SHORT_NAMES;
