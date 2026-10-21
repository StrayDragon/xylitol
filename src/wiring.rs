//! Root façade over `xylitol-kernel`. Occupancy extras that need the TUI client
//! stay here because the kernel crate graph cannot depend on `xylitol-app-tui`.

pub use xylitol_kernel::{
    BootOptions, Kernel, Profile, admit, assembly, attach, boot, boot_ctx, bootstrap, builtins,
    composition, core, dispatch, driver, event, export, mcp_spec, product_commands, provider,
};
pub use xylitol_kernel::{config, obs, settings, timing};

#[cfg(feature = "server")]
pub mod occupancy;
