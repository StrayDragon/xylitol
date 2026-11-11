use std::path::PathBuf;
use std::sync::Arc;

use schemars::JsonSchema;
use serde::Deserialize;
use xylitol_plugin_sdk::XySessionStore;
use xylitol_plugin_sdk::rt::{Ctx, Plugin, Result};

use crate::LogSessionStore;

#[derive(Clone, Debug, Default, Deserialize, JsonSchema)]
#[serde(default)]
pub struct SessionPluginConfig {
    pub sessions_dir: Option<String>,
    pub in_memory: bool,
    pub storage: Option<String>,
}

pub struct SessionStorePlugin {
    config: SessionPluginConfig,
}

impl SessionStorePlugin {
    pub fn new(config: &SessionPluginConfig) -> Result<Self> {
        Ok(Self {
            config: config.clone(),
        })
    }
}

impl Plugin for SessionStorePlugin {
    type Deps = ();

    fn name(&self) -> &str {
        "session-store"
    }

    async fn apply(&self, ctx: &Ctx, _: ()) -> Result {
        let memory_backend =
            self.config.in_memory || self.config.storage.as_deref() == Some("memory");
        let store: Arc<dyn XySessionStore> = if memory_backend {
            Arc::new(LogSessionStore::in_memory())
        } else {
            let dir = self
                .config
                .sessions_dir
                .as_deref()
                .map(PathBuf::from)
                .unwrap_or_else(LogSessionStore::default_dir);
            Arc::new(LogSessionStore::new(dir))
        };
        ctx.provide_as::<dyn XySessionStore>(store)?;
        Ok(())
    }
}
