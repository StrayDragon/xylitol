//! HTTP/WS carrier and `XyEvent` ↔ v3 mapping.
//!
//! Envelope types live in `xylitol-protocol`. Domain events live in
//! `xylitol-runtime`. This crate is the only place that maps between them.

pub mod mapping;

pub use mapping::{FromWireEvent, ToWireEvent, WireEventConvertError};
