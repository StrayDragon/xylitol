//! Session persistence — implementation lives in `xylitol-official-plugin-session`.

pub use xylitol_official_plugin_session::{
    SESSION_WRITE_LOCK_FILENAME, SessionBackend, SessionManager,
};

pub mod types {
    pub use xylitol_official_plugin_session::*;
}

pub mod manager {
    pub use xylitol_official_plugin_session::{SESSION_WRITE_LOCK_FILENAME, SessionManager};
}

pub use types::*;
