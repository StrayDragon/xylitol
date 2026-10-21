//! Import firewall for rutis. Plugin authors depend on this module, not on `rutis`.

pub use rutis::{
    BoxFuture, Call, CordisError, Ctx, DisposeWaitError, Effect, ErrorSink, Event, EventBus,
    EventKey, EventOptions, EventPattern, FiberState, FiberView, Listener, Plugin, PluginFactory,
    PluginId, Snapshot, SyncCall, SyncListener, Terminal, TypeKey, Typed, TypedFactory,
    TypedPlugin, TypedPluginFactory,
};

/// sdk public API version. Bumps when events, service keys, or `rt` types break.
pub const SDK_API_VERSION: &str = "0.2.0";

/// Locked rutis crate version re-exported through this sdk.
pub const RUTIS_LOCK: &str = "0.13.0";
