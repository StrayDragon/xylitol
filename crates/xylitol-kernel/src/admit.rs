use std::sync::Arc;

use rutis::host::BoxFuture;
use rutis_loader::resolve::Resolved;
use rutis_loader::{LoaderError, Resolver};

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

pub const RESERVED_PLUGIN_NAMES: &[&str] = &[
    "config",
    "obs",
    "trust",
    "registries",
    "session-host",
    "agent-react",
    "tool-table",
    "driver",
    "session-store",
    "compaction",
    "tools-builtin",
    "tools-fff",
    "mcp",
    "hooks",
    "loop-policies",
    "todo",
    "workflow",
    "plan",
    "provider-openai",
    "provider-anthropic",
    "provider-fake",
    "app-print",
    "serve",
];

pub fn is_core_plugin_name(name: &str) -> bool {
    CORE_PLUGIN_NAMES.contains(&name)
}

pub fn is_reserved_plugin_name(name: &str) -> bool {
    RESERVED_PLUGIN_NAMES.contains(&name)
}

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
