pub mod cli;
pub mod driver;
pub mod print;
pub mod resources;
pub mod rpc;

/// Interactive diff review (terminal ratatui backend).
/// Requires the `ui-review` feature.
#[cfg(feature = "ui-review")]
pub mod diff_review;
