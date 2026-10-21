//! `PluginFactory` entry: JSONL session store.

use std::path::PathBuf;
use std::sync::Arc;

use schemars::JsonSchema;
use serde::Deserialize;
use xylitol_plugin_sdk::rt::{BoxFuture, CordisError, Ctx, Effect, Plugin, PluginFactory};
use xylitol_plugin_sdk::{XySessionStore, session_store_key};

use crate::SessionManager;

#[derive(Clone, Debug, Default, Deserialize, JsonSchema)]
#[serde(default)]
pub struct SessionPluginConfig {
    pub sessions_dir: Option<String>,
    pub in_memory: bool,
}

pub struct SessionStoreFactory;

impl PluginFactory<SessionPluginConfig> for SessionStoreFactory {
    fn name(&self) -> &str {
        "session-store"
    }

    fn build(&self, config: &SessionPluginConfig) -> Result<Box<dyn Plugin>, CordisError> {
        Ok(Box::new(SessionStorePlugin {
            config: config.clone(),
        }))
    }
}

struct SessionStorePlugin {
    config: SessionPluginConfig,
}

impl Plugin for SessionStorePlugin {
    fn name(&self) -> &str {
        "session-store"
    }

    fn apply<'a>(&'a self, ctx: &'a Ctx) -> BoxFuture<'a, Result<Effect, CordisError>> {
        Box::pin(async move {
            let store: Arc<dyn XySessionStore> = if self.config.in_memory {
                Arc::new(SessionManager::in_memory())
            } else {
                let dir = self
                    .config
                    .sessions_dir
                    .as_deref()
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("."));
                Arc::new(SessionManager::new(dir))
            };
            ctx.provide_as(session_store_key(), store)?;
            Ok(Effect::Done)
        })
    }
}
