//! BDD tests for Xylitol core — rstest-bdd.
//! Run: `cargo test --test bdd`
//!
//! Most scenarios are parallel-safe. A few still mutate process env
//! (`HF_ENDPOINT` download paths, `HOME`/`XYLITOL_*` loader paths) and are
//! gated with `#[serial_test::serial(...)]` on their bindings.

#[macro_use]
mod helpers;
mod bindings_agent_hooks;
mod bindings_agent_runtime;
mod bindings_agent_session;
mod bindings_agent_session_store;
mod bindings_agent_tools;
mod bindings_app_tui;
mod bindings_app_tui_ask;
mod bindings_app_tui_interaction;
mod bindings_app_tui_transcript;
mod bindings_cli_entry;
mod bindings_domain_compaction;
mod bindings_domain_security;
mod bindings_misc;
mod bindings_package_tui_interaction_modes;
mod bindings_protocol_app;
mod bindings_runtime_config;
mod bindings_runtime_model_registry;
mod bindings_server;
mod fixtures;
mod prelude;
mod steps_agent;
mod steps_agent_runtime;
mod steps_agent_session_extra;
mod steps_app_tui_ask;
mod steps_app_tui_interaction;
mod steps_app_tui_transcript;
mod steps_bridge;
mod steps_cli_surface;
mod steps_compaction;
mod steps_domain_compaction_extra;
mod steps_domain_security;
mod steps_hooks;
mod steps_package_tui_interaction_modes;
mod steps_protocol;
mod steps_runtime_config;
mod steps_sandbox;
mod steps_server;
mod steps_session;
mod steps_shared_thens;
mod steps_tokenizer;
mod steps_tools;
mod steps_workspace;

pub use fixtures::*;
pub use steps_app_tui_interaction::{TuiInteraction, tui_interaction};
pub use steps_app_tui_transcript::{TranscriptBdd, transcript_bdd};
pub use steps_bridge::{AiBridgeBdd, PromptBdd, ai_bridge_bdd, prompt_bdd};
pub use steps_protocol::{ProtocolBdd, protocol_bdd};
pub use steps_runtime_config::{RcSnap, rc_snap};
pub use steps_server::{ServerTest, approval_test, server_test};
