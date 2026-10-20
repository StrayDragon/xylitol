//! Server module — Host occupancy + assemble. Listen lives in `xylitol-serve`.
//!
//! Product carrier is v3 fory: `POST /rpc` unary + `WS /rpc` downlink.
//! Occupancy is a **session slot** (journal + seq + writer Driver), not a
//! process-wide `Mutex` Driver.
//! The Host process shares one `RuntimeBaseline` baseline; slots lazy-materialize
//! isolated Drivers. Print / embed still use in-process Driver and do not bind.

#[cfg(feature = "server")]
pub mod host;

#[cfg(feature = "server")]
pub mod idempotency;

#[cfg(feature = "server")]
pub mod rpc_module;

#[cfg(feature = "server")]
pub mod runtime;

#[cfg(feature = "server")]
pub mod subcommand;

#[cfg(feature = "server")]
pub mod envelope;

#[cfg(feature = "server")]
pub mod ws;
