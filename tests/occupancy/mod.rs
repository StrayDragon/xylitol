//! Occupancy glue that must see both Host and product TUI.
//!
//! Kernel / serve cannot depend on `xylitol-app-tui`. These files live under
//! `tests/` and are included from the xylitol crate as `#[cfg(test)]` so they
//! do not add another integration-test binary.

mod assembly_fixture_tests;
mod in_process_host;
