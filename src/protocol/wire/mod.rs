//! Wire protocol — client ↔ core `Command` / `Event` vocabulary.
//!
//! Transport-agnostic: the same types are spoken over stdio RPC, WebSocket, and REST.
//! MUST NOT depend on [`crate::protocol::ports`].

pub mod command;
pub mod event;
pub mod transport;

pub use command::Command;
pub use event::Event;
pub use transport::{Envelope, ErrorCode};
