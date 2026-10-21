pub use xylitol_kernel::boot::{BootOptions, Kernel, Profile, boot};
pub use xylitol_kernel::bootstrap::{
    BootstrapError, BootstrapInput, BootstrapWarning, BootstrappedAgent, BootstrappedRuntime,
    bootstrap,
};
pub use xylitol_kernel::composition::{
    BuildAgentOptions, McpServerSpec, McpSession, McpTransportSpec, build_agent,
};
pub use xylitol_kernel::driver::{
    CommandInfo, EventStream, ModelInfo, SessionState, SessionStats, XyDriver, XyDriverError,
    XyInProcessDriver, assemble_driver,
};
