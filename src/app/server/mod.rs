//! Server module — Host composition root.
//!
//! Product carrier is four-quadrant RPC: HTTP POST unary / respond, plus a
//! downlink-only WebSocket at `GET /api/events.mux`. Occupancy is a **session
//! slot** (journal + seq + writer Driver), not a process-wide `Mutex` Driver.
//! The Host process shares one `RuntimePorts` baseline; slots lazy-materialize
//! isolated Drivers. Print / embed still use in-process Driver and do not bind.

#[cfg(feature = "server")]
pub mod host;

#[cfg(feature = "server")]
pub mod http;

#[cfg(feature = "server")]
pub mod runtime;

#[cfg(feature = "server")]
pub mod subcommand;

pub mod ws;
