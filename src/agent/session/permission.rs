//! Permission gate collaborator — holds the advisory [`XyPermission`] port.
//!
//! [`PermissionGate`] is an advisory in-process filter, NOT a security
//! boundary. The ReAct loop borrows the engine via [`PermissionGate::get`] to
//! perform tool_name → capability routing (see `react.rs`); it does not consult
//! the removed `check_permission_*` wrappers.

use std::sync::Arc;

use crate::runtime_protocol::XyPermission;

/// Stateful permission collaborator — owns the advisory [`XyPermission`] port.
pub struct PermissionGate {
    engine: Arc<dyn XyPermission>,
}

impl PermissionGate {
    /// Construct with a permission engine port.
    pub fn new(engine: Arc<dyn XyPermission>) -> Self {
        Self { engine }
    }

    /// Get a reference to the permission engine (injected at construction).
    ///
    /// The ReAct loop uses this to route tool_name → check_read/write/network.
    pub fn get(&self) -> Arc<dyn XyPermission> {
        self.engine.clone()
    }

    /// Replace the permission engine port. Takes effect on the next turn.
    pub fn set(&mut self, engine: Arc<dyn XyPermission>) {
        self.engine = engine;
    }
}
