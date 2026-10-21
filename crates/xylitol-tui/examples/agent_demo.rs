#[path = "agent_demo_impl.rs"]
mod agent_demo_impl;

use xylitol_tui::InteractionMode;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    agent_demo_impl::run(InteractionMode::Inline)
}
