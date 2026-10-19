//! Wire protocol v3 — fory binary envelope. Schema and generated types live
//! in `xylitol-protocol`; XyEvent mapping stays in this crate until W2.4.

pub use xylitol_protocol::v3::generated;
pub use xylitol_protocol::v3::*;

pub mod mapping;
