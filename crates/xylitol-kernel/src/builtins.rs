use rutis_loader::Builtins;
use xylitol_official_plugin_compaction::plugin::CompactionPlugin;
use xylitol_official_plugin_dap::DapPlugin;
use xylitol_official_plugin_hooks::plugin::HooksPlugin;
use xylitol_official_plugin_loop_policies::plugin::LoopPoliciesPlugin;
use xylitol_official_plugin_lsp::LspPlugin;
use xylitol_official_plugin_mcp::plugin::McpPlugin;
use xylitol_official_plugin_plan::plugin::PlanPlugin;
use xylitol_official_plugin_provider_anthropic::plugin::AnthropicPlugin;
use xylitol_official_plugin_provider_openai::fake_plugin::FakePlugin;
use xylitol_official_plugin_provider_openai::plugin::OpenAiPlugin;
use xylitol_official_plugin_session::plugin::SessionStorePlugin;
use xylitol_official_plugin_todo::plugin::TodoPlugin;
use xylitol_official_plugin_tools::plugin::ToolsPlugin;
#[cfg(feature = "fff")]
use xylitol_official_plugin_tools_fff::plugin::FffToolsPlugin;
use xylitol_official_plugin_workflow::plugin::WorkflowPlugin;

use crate::surfaces::{PrintPlugin, ServePlugin};

pub fn official_builtins() -> Builtins {
    let mut builtins = Builtins::new();
    builtins.register("session-store", SessionStorePlugin::new);
    builtins.register("compaction", CompactionPlugin::new);
    builtins.register("tools-builtin", ToolsPlugin::new);
    #[cfg(feature = "fff")]
    builtins.register("tools-fff", FffToolsPlugin::new);
    builtins.register("mcp", McpPlugin::new);
    builtins.register("hooks", HooksPlugin::new);
    builtins.register("loop-policies", LoopPoliciesPlugin::new);
    builtins.register("todo", TodoPlugin::new);
    builtins.register("workflow", WorkflowPlugin::new);
    builtins.register("plan", PlanPlugin::new);
    builtins.register("lsp", LspPlugin::new);
    builtins.register("dap", DapPlugin::new);
    builtins.register("provider-openai", OpenAiPlugin::new);
    builtins.register("provider-anthropic", AnthropicPlugin::new);
    builtins.register("provider-fake", FakePlugin::new);
    builtins.register("app-print", PrintPlugin::new);
    builtins.register("serve", ServePlugin::new);
    builtins
}
