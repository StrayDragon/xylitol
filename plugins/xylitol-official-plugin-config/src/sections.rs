use schemars::schema_for;

use super::types::{
    AgentsConfig, ExecutionConfig, ExternalPluginConfig, ModelsConfig, SecurityConfig, ServeConfig,
    SessionConfig, ToolBatchConfig, TrustConfig, TuiConfig,
};

use xylitol_plugin_sdk::config_section::ConfigSection;

pub fn core_sections() -> &'static [ConfigSection] {
    CORE_SECTIONS
}

const CORE_SECTIONS: &[ConfigSection] = &[
    ConfigSection {
        key: "models",
        schema: schema_models,
        default: default_models,
    },
    ConfigSection {
        key: "agents",
        schema: schema_agents,
        default: default_agents,
    },
    ConfigSection {
        key: "execution",
        schema: schema_execution,
        default: default_execution,
    },
    ConfigSection {
        key: "security",
        schema: schema_security,
        default: default_security,
    },
    ConfigSection {
        key: "session",
        schema: schema_session,
        default: default_null,
    },
    ConfigSection {
        key: "tui",
        schema: schema_tui,
        default: default_tui,
    },
    ConfigSection {
        key: "tool_batch",
        schema: schema_tool_batch,
        default: default_tool_batch,
    },
    ConfigSection {
        key: "tools",
        schema: schema_tools,
        default: default_tools,
    },
    ConfigSection {
        key: "external_plugins",
        schema: schema_external_plugins,
        default: default_external_plugins,
    },
    ConfigSection {
        key: "subagent",
        schema: schema_subagent,
        default: default_null,
    },
    ConfigSection {
        key: "serve",
        schema: schema_serve,
        default: default_serve,
    },
    ConfigSection {
        key: "trust",
        schema: schema_trust,
        default: default_trust,
    },
];

fn schema_models() -> serde_json::Value {
    serde_json::to_value(schema_for!(ModelsConfig)).expect("models schema")
}
fn schema_agents() -> serde_json::Value {
    serde_json::to_value(schema_for!(AgentsConfig)).expect("agents schema")
}
fn schema_execution() -> serde_json::Value {
    serde_json::to_value(schema_for!(ExecutionConfig)).expect("execution schema")
}
fn schema_security() -> serde_json::Value {
    serde_json::to_value(schema_for!(SecurityConfig)).expect("security schema")
}
fn schema_session() -> serde_json::Value {
    serde_json::to_value(schema_for!(SessionConfig)).expect("session schema")
}
fn schema_tui() -> serde_json::Value {
    serde_json::to_value(schema_for!(TuiConfig)).expect("tui schema")
}
fn schema_tool_batch() -> serde_json::Value {
    serde_json::to_value(schema_for!(ToolBatchConfig)).expect("tool_batch schema")
}
fn schema_tools() -> serde_json::Value {
    serde_json::to_value(schema_for!(Vec<String>)).expect("tools schema")
}
fn schema_external_plugins() -> serde_json::Value {
    serde_json::to_value(schema_for!(Vec<ExternalPluginConfig>)).expect("external_plugins schema")
}
fn schema_subagent() -> serde_json::Value {
    serde_json::to_value(schema_for!(super::types::SubagentConfig)).expect("subagent schema")
}
fn schema_serve() -> serde_json::Value {
    serde_json::to_value(schema_for!(ServeConfig)).expect("serve schema")
}
fn schema_trust() -> serde_json::Value {
    serde_json::to_value(schema_for!(TrustConfig)).expect("trust schema")
}

fn default_null() -> serde_json::Value {
    serde_json::Value::Null
}
fn default_models() -> serde_json::Value {
    serde_json::to_value(ModelsConfig::default()).expect("models default")
}
fn default_agents() -> serde_json::Value {
    serde_json::to_value(AgentsConfig::default()).expect("agents default")
}
fn default_execution() -> serde_json::Value {
    serde_json::to_value(ExecutionConfig::default()).expect("execution default")
}
fn default_security() -> serde_json::Value {
    serde_json::to_value(SecurityConfig::default()).expect("security default")
}
fn default_tui() -> serde_json::Value {
    serde_json::to_value(TuiConfig::default()).expect("tui default")
}
fn default_tool_batch() -> serde_json::Value {
    serde_json::to_value(ToolBatchConfig::default()).expect("tool_batch default")
}
fn default_tools() -> serde_json::Value {
    serde_json::json!([])
}
fn default_external_plugins() -> serde_json::Value {
    serde_json::json!([])
}
fn default_serve() -> serde_json::Value {
    serde_json::to_value(ServeConfig::default()).expect("serve default")
}
fn default_trust() -> serde_json::Value {
    serde_json::to_value(TrustConfig::default()).expect("trust default")
}
