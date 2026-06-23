//! # xylitol
//!
//! LLM-Augmented Development Toolkit.

pub mod agent;
pub mod core;
pub mod infra;
pub mod interface;

/// Application entry point.
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    interface::cli::run().await
}

/// Run the interactive diff review demo.
pub async fn run_review_demo() -> Result<(), String> {
    interface::diff_review::run_demo().await
}

#[cfg(test)]
mod tests;
