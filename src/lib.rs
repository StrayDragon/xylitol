//! # xylitol
//!
//! LLM-Augmented Development Toolkit.

pub mod agent;
pub mod core;
pub mod infra;
pub mod interactive;

/// Application entry point.
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    interactive::cli::run().await
}

/// Run the interactive diff review demo.
///
/// Requires the `ui-review` feature (terminal rendering backend).
#[cfg(feature = "ui-review")]
pub async fn run_review_demo() -> Result<(), String> {
    interactive::diff_review::run_demo().await
}

#[cfg(test)]
mod tests;
