use std::path::PathBuf;
use std::sync::Arc;

use schemars::JsonSchema;
use serde::Deserialize;
use xylitol_plugin_sdk::XySessionStore;
use xylitol_plugin_sdk::rt::{Ctx, Plugin, Result};

use crate::SessionManager;

#[derive(Clone, Debug, Default, Deserialize, JsonSchema)]
#[serde(default)]
pub struct SessionPluginConfig {
    pub sessions_dir: Option<String>,
    pub in_memory: bool,
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
        let store: Arc<dyn XySessionStore> = if self.config.in_memory {
            Arc::new(SessionManager::in_memory())
        } else {
            let dir = self
                .config
                .sessions_dir
                .as_deref()
                .map(PathBuf::from)
                .unwrap_or_else(SessionManager::default_dir);
            Arc::new(SessionManager::new(dir))
        };
        ctx.provide_as::<dyn XySessionStore>(store)?;
        Ok(())
    }
}
