//! Mount admission: core plugin names may not appear in loader rows.

use std::sync::Arc;

use rutis_loader::{LoaderError, Resolved, Resolver};
use xylitol_plugin_sdk::rt::BoxFuture;

/// Names the kernel mounts itself. Rows using these names are rejected.
pub const CORE_PLUGIN_NAMES: &[&str] = &[
    "config",
    "obs",
    "trust",
    "registries",
    "session-host",
    "agent-react",
    "tool-table",
    "driver",
];

pub fn is_core_plugin_name(name: &str) -> bool {
    CORE_PLUGIN_NAMES.contains(&name)
}

/// Wraps a resolver and refuses core names with a non-NotFound error
/// so a [`rutis_loader::Chain`] cannot fall through to a later ring.
pub struct Admit<R> {
    pub inner: R,
}

impl<R: Resolver> Resolver for Admit<R> {
    fn resolve<'a>(&'a self, name: &'a str) -> BoxFuture<'a, Result<Arc<Resolved>, LoaderError>> {
        Box::pin(async move {
            if is_core_plugin_name(name) {
                return Err(LoaderError::Resolve {
                    name: name.to_owned(),
                    message: "core plugin names cannot be mounted from rows".into(),
                });
            }
            self.inner.resolve(name).await
        })
    }
}
