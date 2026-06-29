//! Protocol — the single source of truth for client↔core interaction.
//!
//! Defines the command (client → core) and event (core → client) vocabularies.
//! Transport-agnostic: the same [`Command`]/[`Event`] types are spoken over the
//! stdio RPC transport, WebSocket, and REST.
//!
//! Wire format is stable: serde `tag = "type"` + `snake_case` variants. Adding
//! a command/event = adding a variant; unknown variants are tolerated by serde
//! defaults on the receiver side.

pub mod command;
pub mod event;
pub mod transport;

pub use command::Command;
pub use event::Event;
pub use transport::{Envelope, ErrorCode};
