use std::sync::Arc;

use xylitol_plugin_sdk::compaction_config::XyCompactionSettingsConfig;
use xylitol_plugin_sdk::rt::{BoxFuture, CordisError, Ctx, Effect, Plugin, PluginFactory};
use xylitol_plugin_sdk::{CompactionSettings, XyCompaction, compaction_key};

use crate::CompactionOrchestrator;

pub struct CompactionFactory;

impl PluginFactory<XyCompactionSettingsConfig> for CompactionFactory {
    fn name(&self) -> &str {
        "compaction"
    }

    fn build(&self, config: &XyCompactionSettingsConfig) -> Result<Box<dyn Plugin>, CordisError> {
        Ok(Box::new(CompactionPlugin {
            settings: CompactionSettings::from(config.clone()),
        }))
    }
}

struct CompactionPlugin {
    settings: CompactionSettings,
}

impl Plugin for CompactionPlugin {
    fn name(&self) -> &str {
        "compaction"
    }

    fn apply<'a>(&'a self, ctx: &'a Ctx) -> BoxFuture<'a, Result<Effect, CordisError>> {
        Box::pin(async move {
            let orch: Arc<dyn XyCompaction> =
                Arc::new(CompactionOrchestrator::new(self.settings.clone()));
            ctx.provide_as(compaction_key(), orch)?;
            Ok(Effect::Done)
        })
    }
}
