//! Package interactive harness — **ApplicationOwned** (alt-screen) entry.
//!
//! Shared app lives in [`agent_demo_impl`]. Inline uses `agent_demo`.
//! Do **not** switch modes via `XYLITOL_AGENT_DEMO_MODE`.

#[path = "agent_demo_impl.rs"]
mod agent_demo_impl;

use xylitol_tui::InteractionMode;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    agent_demo_impl::run(InteractionMode::ApplicationOwned)
}
