use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Clone, Debug, Default, Deserialize, JsonSchema)]
pub struct EmptyPluginConfig {}
