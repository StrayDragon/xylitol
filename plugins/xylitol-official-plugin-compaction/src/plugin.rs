use std::sync::Arc;

use xylitol_plugin_sdk::compaction_config::XyCompactionSettingsConfig;
use xylitol_plugin_sdk::rt::{Ctx, Plugin, Result};
use xylitol_plugin_sdk::{CompactionSettings, XyCompaction};

use crate::CompactionOrchestrator;

pub struct CompactionPlugin {
    settings: CompactionSettings,
}

impl CompactionPlugin {
    pub fn new(config: &XyCompactionSettingsConfig) -> Result<Self> {
        Ok(Self {
            settings: CompactionSettings::from(config.clone()),
        })
    }
}

impl Plugin for CompactionPlugin {
    type Deps = ();

    fn name(&self) -> &str {
        "compaction"
    }

    async fn apply(&self, ctx: &Ctx, _: ()) -> Result {
        let orch: Arc<dyn XyCompaction> =
            Arc::new(CompactionOrchestrator::new(self.settings.clone()));
        ctx.provide_as::<dyn XyCompaction>(orch)?;
        Ok(())
    }
}
