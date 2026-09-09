//! Prompt/input shaping — system prompt assembly. The agent loop consumes
//! these to build the input it sends to the model.
//!
//! Product slash catalogs live in `app::product_commands` (not here).

pub mod fragments;
pub(crate) mod sandbox;
pub mod session_env;
pub mod skill_expand;
pub mod system;
// Umbrella re-export: system-prompt construction is the subsystem's main entry.
pub(crate) use fragments::{fragment_ids_for_batch_mode, fragments_for_batch_mode};
#[cfg(test)]
pub use session_env::session_env_from_message;
pub use session_env::{ensure_session_env_in_history, snapshot_for_cwd};
pub(crate) use skill_expand::expand_skills_in_agent_messages;
pub use system::{SystemPromptOpts, build_system_prompt};
pub(crate) use system::{collect_tool_guidelines, collect_tool_snippets};
