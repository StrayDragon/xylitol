//! # xylitol
//!
//! LLM-Augmented Development Toolkit.

pub mod agent;
pub mod app;
pub mod domain;
pub mod infra;
pub mod protocol;
pub mod runtime_protocol;

/// Application entry point.
#[cfg(feature = "cli")]
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    app::cli::run().await
}

/// Run the interactive diff review demo.
///
/// Requires the `tui` feature (terminal rendering backend).
#[cfg(feature = "tui")]
pub async fn run_review_demo() -> Result<(), String> {
    app::tui::diff_review::run_demo().await
}

#[cfg(test)]
mod tests;
