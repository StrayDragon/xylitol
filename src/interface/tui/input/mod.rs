//! TUI input layer — three-layer keyboard model.
//!
//! Layer 1: decode crossterm events into InputKey (input/decode.rs)
//! Layer 2: map (InputKey, FocusCtx, AgentState) → Option<Action> (input/keymap.rs)
//! Layer 3: apply Action to App state (input/action.rs)

pub(crate) mod action;
pub(crate) mod decode;
pub(crate) mod keymap;
