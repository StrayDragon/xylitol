//! # xylitol
//!
//! LLM-Augmented Development Toolkit.

#![allow(dead_code)]
#![cfg_attr(docsrs, warn(missing_docs))]

pub(crate) mod agent;
pub(crate) mod infra;
pub(crate) mod interface;

/// Application entry point — parses CLI args, loads config, dispatches to mode.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    interface::cli::run()
}

/// Run the interactive diff review demo (requires `ui-review` feature).
#[cfg(feature = "ui-review")]
pub async fn run_review_demo() -> Result<(), String> {
    interface::diff_review::run_demo().await
}
