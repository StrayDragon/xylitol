use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use adk_session::{InMemorySessionService, SessionService};

/// In-memory session manager for tests.
pub(crate) struct SessionManager;

impl SessionManager {
    pub(crate) fn in_memory() -> Arc<dyn SessionService> {
        Arc::new(InMemorySessionService::new())
    }
}

/// Minimal in-memory settings manager placeholder for tests.
///
/// This is intentionally small today; it exists to standardize "no-FS" test wiring.
pub(crate) struct SettingsManager {
    _kv: Mutex<HashMap<String, String>>,
}

impl SettingsManager {
    pub(crate) fn in_memory() -> Self {
        Self {
            _kv: Mutex::new(HashMap::new()),
        }
    }
}

/// Minimal in-memory auth storage placeholder for tests.
pub(crate) struct AuthStorage {
    _kv: Mutex<HashMap<String, String>>,
}

impl AuthStorage {
    pub(crate) fn in_memory() -> Self {
        Self {
            _kv: Mutex::new(HashMap::new()),
        }
    }
}
