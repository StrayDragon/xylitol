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

#[tokio::main]
async fn main() -> Result<(), String> {
    xylitol::run_review_demo().await
}
