//! HTTP/WS carrier and `XyEvent` ↔ v3 mapping.
//!
//! Envelope types live in `xylitol-protocol`. Domain events live in
//! `xylitol-runtime`. This crate is the only place that maps between them.
//! Occupancy is injected via [`listen::OccupiedHost`]; this crate does not
//! assemble Host plugins.

pub mod mapping;
pub mod v3_mapping;

pub use mapping::{FromWireEvent, ToWireEvent, WireEventConvertError};

#[cfg(feature = "listen")]
pub mod listen;
#[cfg(feature = "listen")]
pub mod registration;

#[cfg(feature = "listen")]
pub use listen::{
    CONTENT_TYPE, Gateway, ListenConfig, MUX_CHAN_CAP, MuxSink, OccupiedHost, Phase,
    RunningListener, bind_listen, looks_like_fory_envelope, router, serve, serve_registered,
};
#[cfg(feature = "listen")]
pub use registration::{
    REGISTRATION_FILE_NAME, Registration, default_registration_path, read_registration,
    remove_registration, run_self_check, write_registration,
};
