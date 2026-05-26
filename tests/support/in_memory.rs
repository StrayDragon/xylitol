use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::agent::session::{InMemorySession, XySession};

/// In-memory session manager for tests.
pub(crate) struct SessionManager;

impl SessionManager {
    pub(crate) fn in_memory() -> Arc<dyn XySession> {
        Arc::new(InMemorySession::new())
    }
}

/// Minimal in-memory settings manager placeholder for tests.
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
