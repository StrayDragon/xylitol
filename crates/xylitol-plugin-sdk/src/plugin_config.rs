//! Shared loader config types for official and future third-party plugins.

use schemars::JsonSchema;
use serde::Deserialize;

/// Empty JSON object `{}` / `null` config for plugins with no parameters.
#[derive(Clone, Debug, Default, Deserialize, JsonSchema)]
pub struct EmptyPluginConfig {}
