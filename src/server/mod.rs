//! Server module — hosts agent + infra runtimes and exposes protocol/ over
//! REST (/api/v1) and WebSocket.
//!
//! This is the second composition root (the first being interactive::cli).

pub mod lock;
pub mod port_retry;
pub mod rest;
pub mod runtime;
pub mod ws;
