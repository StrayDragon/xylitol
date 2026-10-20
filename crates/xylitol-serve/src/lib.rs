//! HTTP/WS carrier and `XyEvent` ↔ v3 mapping.
//!
//! Envelope types live in `xylitol-protocol`. Domain events live in
//! `xylitol-runtime`. This crate is the only place that maps between them.
//! Occupancy policy lives in [`occupancy`]; assembly (plugins / MCP) is injected
//! via [`occupancy::OccupancyFactory`]. This crate does not assemble Host plugins.

pub mod mapping;
pub mod v3_mapping;

pub use mapping::{FromWireEvent, ToWireEvent, WireEventConvertError};

#[cfg(feature = "listen")]
pub mod listen;
#[cfg(feature = "listen")]
pub mod occupancy;
#[cfg(feature = "listen")]
pub mod registration;

#[cfg(feature = "listen")]
pub use listen::{
    CONTENT_TYPE, Gateway, ListenConfig, MUX_CHAN_CAP, MuxSink, OccupiedHost, Phase,
    RunningListener, bind_listen, looks_like_fory_envelope, router, serve, serve_registered,
};
#[cfg(feature = "listen")]
pub use occupancy::{HostState, OccupancyEngine, OccupancyFactory, ReloadBaseline};
#[cfg(feature = "listen")]
pub use registration::{
    REGISTRATION_FILE_NAME, Registration, default_registration_path, read_registration,
    remove_registration, run_self_check, write_registration,
};
