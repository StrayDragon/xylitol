pub mod config;
pub mod coordinator;
pub mod drift;
pub mod plugin;
pub mod registry;

pub use config::{ContextInjectConfig, DedupMode, InjectorConfig, InjectorPoint, Placement};
pub use coordinator::ContextInjectMiddleware;
pub use drift::{CUSTOM_TYPE_CONTEXT_FINGERPRINT, DriftInjector, FingerprintSnapshot};
pub use plugin::ContextInjectPlugin;
pub use registry::{
    ContextInjector, ContextInjectorRegistry, InjectorContext, InjectorSource, NativeInjector,
};
