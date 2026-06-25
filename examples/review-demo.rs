//! xylitol Diff Review Demo
//!
//! Generates a realistic temp project, simulates agent modifications, then
//! opens the interactive diff review (CLI ratatui or Web Monaco Editor).
//!
//! # Usage
//!
//! ```bash
//! cargo run --example review-demo                    # CLI review (default)
//! REVIEW_BACKEND=web cargo run --example review-demo  # Web review
//! ```
//!
//! CLI keys: j/k navigate, c comment, a accept, r reject, ? help, q quit.

#[cfg(feature = "ui-review")]
#[tokio::main]
async fn main() -> Result<(), String> {
    xylitol::interface::diff_review::run_demo().await
}

#[cfg(not(feature = "ui-review"))]
#[tokio::main]
async fn main() -> Result<(), String> {
    println!("Review demo requires the `ui-review` feature.");
    println!("Run with: cargo run --example review-demo --features ui-review");
    Ok(())
}
