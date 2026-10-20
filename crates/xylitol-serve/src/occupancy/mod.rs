//! Host occupancy: session slots, linger, single writer, mux journal.
//!
//! Assembly (official plugins / MCP) stays in the composition root and is
//! injected via [`OccupancyFactory`].

mod engine;
mod envelope;
mod host;
mod idempotency;
mod rpc_module;
mod ws;

pub use engine::{OccupancyEngine, OccupancyFactory};
pub use envelope::handle_uplink;
pub use host::{
    HostState, ReloadBaseline, SessionSlot, handle_unary, materialize_writer,
    materialize_writer_at, prune_mux_and_maybe_release,
};
pub use rpc_module::dispatch_raw;
pub use ws::{EventJournal, ReverseRpcResult};
