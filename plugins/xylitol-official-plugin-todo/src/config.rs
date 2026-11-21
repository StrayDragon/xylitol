use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use xylitol_plugin_sdk::config_section::SectionBinding;

pub const DEFAULT_TODO_REMINDER_STALE_AFTER: u32 = 3;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct TodoConfig {
    pub enabled: bool,
    pub progress: bool,
    pub reminder: bool,
    pub reminder_stale_after: u32,
    pub checkpoint: bool,
    pub max_settles: Option<u32>,
}

impl Default for TodoConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            progress: false,
            reminder: false,
            reminder_stale_after: DEFAULT_TODO_REMINDER_STALE_AFTER,
            checkpoint: false,
            max_settles: None,
        }
    }
}

impl TodoConfig {
    pub fn is_active(&self) -> bool {
        self.enabled
    }
}

impl SectionBinding for TodoConfig {
    const KEY: &'static str = "todo";

    fn schema_value() -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(TodoConfig)).expect("todo schema")
    }

    fn default_value() -> serde_json::Value {
        serde_json::to_value(TodoConfig::default()).expect("todo default")
    }
}

pub fn config_section() -> xylitol_plugin_sdk::config_section::ConfigSection {
    <TodoConfig as SectionBinding>::descriptor()
}

#[cfg(test)]
mod section_decl_tests {
    use super::*;

    #[test]
    fn todo_section_declares_key_schema_default() {
        assert_eq!(<TodoConfig as SectionBinding>::KEY, "todo");
        let section = config_section();
        assert_eq!(section.key, <TodoConfig as SectionBinding>::KEY);
        assert!((section.schema)().is_object());
        assert_eq!(
            (section.default)(),
            serde_json::to_value(TodoConfig::default()).expect("todo default")
        );
    }
}
