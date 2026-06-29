//! Server module — hosts agent + infra runtimes and exposes protocol/ over
//! REST (/api/v1) and WebSocket.
//!
//! This is the second composition root (the first being app::cli).

pub mod lock;
pub mod port_retry;

#[cfg(feature = "server")]
pub mod rest;

#[cfg(feature = "server")]
pub mod runtime;

pub mod ws;
