//! # xylitol
//!
//! LLM-Augmented Development Toolkit.

#![allow(dead_code)]

pub mod agent;
pub mod infra;
pub mod interface;

/// Application entry point.
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    interface::cli::run().await
}

/// Run the interactive diff review demo (requires `ui-review` feature).
#[cfg(feature = "ui-review")]
pub async fn run_review_demo() -> Result<(), String> {
    interface::diff_review::run_demo().await
}

#[cfg(test)]
mod tests;
