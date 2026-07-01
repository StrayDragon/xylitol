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

#[cfg(test)]
mod tests;
