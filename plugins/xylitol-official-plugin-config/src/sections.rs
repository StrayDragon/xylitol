use schemars::schema_for;

use super::types::{
    AgentsConfig, DapPluginConfig, ExecutionConfig, ExternalPluginConfig, HooksConfig,
    LspPluginConfig, McpServerConfig, ModelsConfig, OtelConfig, SecurityConfig, ServeConfig,
    SessionConfig, SubagentConfig, ToolBatchConfig, TrustConfig, TuiConfig,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigDomain {
    General,
    Agent,
    App,
}

pub struct ConfigSection {
    pub key: &'static str,
    pub domain: ConfigDomain,
    pub schema: fn() -> serde_json::Value,
    pub default: fn() -> serde_json::Value,
}

pub fn sections() -> &'static [ConfigSection] {
    SECTIONS
}

const SECTIONS: &[ConfigSection] = &[
    ConfigSection {
        key: "models",
        domain: ConfigDomain::Agent,
        schema: schema_models,
        default: default_models,
    },
    ConfigSection {
        key: "agents",
        domain: ConfigDomain::Agent,
        schema: schema_agents,
        default: default_agents,
    },
    ConfigSection {
        key: "execution",
        domain: ConfigDomain::Agent,
        schema: schema_execution,
        default: default_execution,
    },
    ConfigSection {
        key: "hooks",
        domain: ConfigDomain::Agent,
        schema: schema_hooks,
        default: default_hooks,
    },
    ConfigSection {
        key: "security",
        domain: ConfigDomain::App,
        schema: schema_security,
        default: default_security,
    },
    ConfigSection {
        key: "session",
        domain: ConfigDomain::Agent,
        schema: schema_session,
        default: default_null,
    },
    ConfigSection {
        key: "compaction",
        domain: ConfigDomain::Agent,
        schema: schema_compaction,
        default: default_null,
    },
    ConfigSection {
        key: "mcp_servers",
        domain: ConfigDomain::Agent,
        schema: schema_mcp_servers,
        default: default_null,
    },
    ConfigSection {
        key: "otel",
        domain: ConfigDomain::App,
        schema: schema_otel,
        default: default_otel,
    },
    ConfigSection {
        key: "tui",
        domain: ConfigDomain::App,
        schema: schema_tui,
        default: default_tui,
    },
    ConfigSection {
        key: "tool_batch",
        domain: ConfigDomain::Agent,
        schema: schema_tool_batch,
        default: default_tool_batch,
    },
    ConfigSection {
        key: "tools",
        domain: ConfigDomain::Agent,
        schema: schema_tools,
        default: default_tools,
    },
    ConfigSection {
        key: "loop_policies",
        domain: ConfigDomain::Agent,
        schema: schema_loop_policies,
        default: default_loop_policies,
    },
    ConfigSection {
        key: "todo",
        domain: ConfigDomain::Agent,
        schema: schema_todo,
        default: default_todo,
    },
    ConfigSection {
        key: "workflow",
        domain: ConfigDomain::Agent,
        schema: schema_workflow,
        default: default_workflow,
    },
    ConfigSection {
        key: "plan",
        domain: ConfigDomain::Agent,
        schema: schema_plan,
        default: default_plan,
    },
    ConfigSection {
        key: "external_plugins",
        domain: ConfigDomain::App,
        schema: schema_external_plugins,
        default: default_external_plugins,
    },
    ConfigSection {
        key: "lsp",
        domain: ConfigDomain::Agent,
        schema: schema_lsp,
        default: default_null,
    },
    ConfigSection {
        key: "dap",
        domain: ConfigDomain::Agent,
        schema: schema_dap,
        default: default_null,
    },
    ConfigSection {
        key: "subagent",
        domain: ConfigDomain::Agent,
        schema: schema_subagent,
        default: default_null,
    },
    ConfigSection {
        key: "serve",
        domain: ConfigDomain::App,
        schema: schema_serve,
        default: default_serve,
    },
    ConfigSection {
        key: "trust",
        domain: ConfigDomain::App,
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
fn schema_hooks() -> serde_json::Value {
    serde_json::to_value(schema_for!(HooksConfig)).expect("hooks schema")
}
fn schema_security() -> serde_json::Value {
    serde_json::to_value(schema_for!(SecurityConfig)).expect("security schema")
}
fn schema_session() -> serde_json::Value {
    serde_json::to_value(schema_for!(SessionConfig)).expect("session schema")
}
fn schema_compaction() -> serde_json::Value {
    serde_json::to_value(schema_for!(
        xylitol_plugin_sdk::compaction_config::XyCompactionSettingsConfig
    ))
    .expect("compaction schema")
}
fn schema_mcp_servers() -> serde_json::Value {
    serde_json::to_value(schema_for!(Vec<McpServerConfig>)).expect("mcp_servers schema")
}
fn schema_otel() -> serde_json::Value {
    serde_json::to_value(schema_for!(OtelConfig)).expect("otel schema")
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
fn schema_loop_policies() -> serde_json::Value {
    serde_json::to_value(schema_for!(super::types::LoopPoliciesConfig))
        .expect("loop_policies schema")
}
fn schema_todo() -> serde_json::Value {
    serde_json::to_value(schema_for!(super::types::TodoConfig)).expect("todo schema")
}
fn schema_workflow() -> serde_json::Value {
    serde_json::to_value(schema_for!(super::types::WorkflowConfig)).expect("workflow schema")
}
fn schema_plan() -> serde_json::Value {
    serde_json::to_value(schema_for!(super::types::PlanConfig)).expect("plan schema")
}
fn schema_external_plugins() -> serde_json::Value {
    serde_json::to_value(schema_for!(Vec<ExternalPluginConfig>)).expect("external_plugins schema")
}
fn schema_lsp() -> serde_json::Value {
    serde_json::to_value(schema_for!(LspPluginConfig)).expect("lsp schema")
}
fn schema_dap() -> serde_json::Value {
    serde_json::to_value(schema_for!(DapPluginConfig)).expect("dap schema")
}
fn schema_subagent() -> serde_json::Value {
    serde_json::to_value(schema_for!(SubagentConfig)).expect("subagent schema")
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
fn default_hooks() -> serde_json::Value {
    serde_json::to_value(HooksConfig::default()).expect("hooks default")
}
fn default_security() -> serde_json::Value {
    serde_json::to_value(SecurityConfig::default()).expect("security default")
}
fn default_otel() -> serde_json::Value {
    serde_json::to_value(OtelConfig::default()).expect("otel default")
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
fn default_loop_policies() -> serde_json::Value {
    serde_json::to_value(super::types::LoopPoliciesConfig::default())
        .expect("loop_policies default")
}
fn default_todo() -> serde_json::Value {
    serde_json::to_value(super::types::TodoConfig::default()).expect("todo default")
}
fn default_workflow() -> serde_json::Value {
    serde_json::to_value(super::types::WorkflowConfig::default()).expect("workflow default")
}
fn default_plan() -> serde_json::Value {
    serde_json::to_value(super::types::PlanConfig::default()).expect("plan default")
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
