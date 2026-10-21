//! Host occupancy assembly: factory + bind. Slot policy lives in `xylitol-serve`.

#[cfg(feature = "server")]
pub mod factory;
#[cfg(feature = "server")]
pub mod runtime;
#[cfg(feature = "server")]
pub mod subcommand;

#[cfg(feature = "server")]
pub use factory::{
    ProductOccupancyFactory, host_state_for_test, host_state_for_test_with_default_model,
    host_state_for_test_with_mcp,
};
#[cfg(feature = "server")]
pub use xylitol_serve::occupancy as host;
#[cfg(feature = "server")]
pub use xylitol_serve::occupancy::{HostState, ReloadBaseline, SessionSlot, handle_unary};
