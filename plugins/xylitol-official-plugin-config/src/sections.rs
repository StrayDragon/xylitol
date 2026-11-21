use std::sync::LazyLock;

use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};

use super::types::{
    AgentsConfig, ExecutionConfig, ModelsConfig, PluginsConfig, SecurityConfig, ServeConfig,
    SessionConfig, SubagentConfig, ToolBatchConfig, TrustConfig, TuiConfig,
};

use xylitol_plugin_sdk::config_section::{ConfigSection, SectionBinding};

pub fn core_sections() -> &'static [ConfigSection] {
    &CORE_SECTIONS
}

static CORE_SECTIONS: LazyLock<Vec<ConfigSection>> = LazyLock::new(|| {
    vec![
        ModelsConfig::descriptor(),
        AgentsConfig::descriptor(),
        ExecutionConfig::descriptor(),
        SecurityConfig::descriptor(),
        SessionConfig::descriptor(),
        TuiConfig::descriptor(),
        ToolBatchConfig::descriptor(),
        ToolsSection::descriptor(),
        PluginsConfig::descriptor(),
        SubagentConfig::descriptor(),
        ServeConfig::descriptor(),
        TrustConfig::descriptor(),
    ]
});

impl SectionBinding for ModelsConfig {
    const KEY: &'static str = "models";

    fn schema_value() -> serde_json::Value {
        serde_json::to_value(schema_for!(ModelsConfig)).expect("models schema")
    }

    fn default_value() -> serde_json::Value {
        serde_json::to_value(ModelsConfig::default()).expect("models default")
    }
}

impl SectionBinding for AgentsConfig {
    const KEY: &'static str = "agents";

    fn schema_value() -> serde_json::Value {
        serde_json::to_value(schema_for!(AgentsConfig)).expect("agents schema")
    }

    fn default_value() -> serde_json::Value {
        serde_json::to_value(AgentsConfig::default()).expect("agents default")
    }
}

impl SectionBinding for ExecutionConfig {
    const KEY: &'static str = "execution";

    fn schema_value() -> serde_json::Value {
        serde_json::to_value(schema_for!(ExecutionConfig)).expect("execution schema")
    }

    fn default_value() -> serde_json::Value {
        serde_json::to_value(ExecutionConfig::default()).expect("execution default")
    }
}

impl SectionBinding for SecurityConfig {
    const KEY: &'static str = "security";

    fn schema_value() -> serde_json::Value {
        serde_json::to_value(schema_for!(SecurityConfig)).expect("security schema")
    }

    fn default_value() -> serde_json::Value {
        serde_json::to_value(SecurityConfig::default()).expect("security default")
    }
}

impl SectionBinding for SessionConfig {
    const KEY: &'static str = "session";

    fn schema_value() -> serde_json::Value {
        serde_json::to_value(schema_for!(SessionConfig)).expect("session schema")
    }

    fn default_value() -> serde_json::Value {
        serde_json::Value::Null
    }
}

impl SectionBinding for TuiConfig {
    const KEY: &'static str = "tui";

    fn schema_value() -> serde_json::Value {
        serde_json::to_value(schema_for!(TuiConfig)).expect("tui schema")
    }

    fn default_value() -> serde_json::Value {
        serde_json::to_value(TuiConfig::default()).expect("tui default")
    }
}

impl SectionBinding for ToolBatchConfig {
    const KEY: &'static str = "tool_batch";

    fn schema_value() -> serde_json::Value {
        serde_json::to_value(schema_for!(ToolBatchConfig)).expect("tool_batch schema")
    }

    fn default_value() -> serde_json::Value {
        serde_json::to_value(ToolBatchConfig::default()).expect("tool_batch default")
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(transparent)]
#[schemars(transparent)]
pub struct ToolsSection(pub Vec<String>);

impl SectionBinding for ToolsSection {
    const KEY: &'static str = "tools";

    fn schema_value() -> serde_json::Value {
        serde_json::to_value(schema_for!(ToolsSection)).expect("tools schema")
    }

    fn default_value() -> serde_json::Value {
        serde_json::json!([])
    }
}

impl SectionBinding for PluginsConfig {
    const KEY: &'static str = "plugins";

    fn schema_value() -> serde_json::Value {
        serde_json::to_value(schema_for!(PluginsConfig)).expect("plugins schema")
    }

    fn default_value() -> serde_json::Value {
        serde_json::to_value(PluginsConfig::default()).expect("plugins default")
    }
}

impl SectionBinding for SubagentConfig {
    const KEY: &'static str = "subagent";

    fn schema_value() -> serde_json::Value {
        serde_json::to_value(schema_for!(SubagentConfig)).expect("subagent schema")
    }

    fn default_value() -> serde_json::Value {
        serde_json::Value::Null
    }
}

impl SectionBinding for ServeConfig {
    const KEY: &'static str = "serve";

    fn schema_value() -> serde_json::Value {
        serde_json::to_value(schema_for!(ServeConfig)).expect("serve schema")
    }

    fn default_value() -> serde_json::Value {
        serde_json::to_value(ServeConfig::default()).expect("serve default")
    }
}

impl SectionBinding for TrustConfig {
    const KEY: &'static str = "trust";

    fn schema_value() -> serde_json::Value {
        serde_json::to_value(schema_for!(TrustConfig)).expect("trust schema")
    }

    fn default_value() -> serde_json::Value {
        serde_json::to_value(TrustConfig::default()).expect("trust default")
    }
}

#[cfg(test)]
mod section_decl_tests {
    use super::*;

    #[test]
    fn core_section_keys_match_original_descriptors() {
        assert_eq!(ModelsConfig::KEY, "models");
        assert_eq!(AgentsConfig::KEY, "agents");
        assert_eq!(ExecutionConfig::KEY, "execution");
        assert_eq!(SecurityConfig::KEY, "security");
        assert_eq!(SessionConfig::KEY, "session");
        assert_eq!(TuiConfig::KEY, "tui");
        assert_eq!(ToolBatchConfig::KEY, "tool_batch");
        assert_eq!(ToolsSection::KEY, "tools");
        assert_eq!(PluginsConfig::KEY, "plugins");
        assert_eq!(SubagentConfig::KEY, "subagent");
        assert_eq!(ServeConfig::KEY, "serve");
        assert_eq!(TrustConfig::KEY, "trust");
        let keys: Vec<&str> = core_sections().iter().map(|section| section.key).collect();
        assert_eq!(
            keys,
            vec![
                ModelsConfig::KEY,
                AgentsConfig::KEY,
                ExecutionConfig::KEY,
                SecurityConfig::KEY,
                SessionConfig::KEY,
                TuiConfig::KEY,
                ToolBatchConfig::KEY,
                ToolsSection::KEY,
                PluginsConfig::KEY,
                SubagentConfig::KEY,
                ServeConfig::KEY,
                TrustConfig::KEY,
            ]
        );
    }
}
