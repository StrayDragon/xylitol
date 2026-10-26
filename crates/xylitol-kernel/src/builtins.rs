use rutis_loader::Builtins;
use xylitol_official_plugin_compaction::plugin::CompactionPlugin;
use xylitol_official_plugin_hooks::plugin::HooksPlugin;
use xylitol_official_plugin_mcp::plugin::McpPlugin;
use xylitol_official_plugin_provider_anthropic::plugin::AnthropicPlugin;
use xylitol_official_plugin_provider_openai::fake_plugin::FakePlugin;
use xylitol_official_plugin_provider_openai::plugin::OpenAiPlugin;
use xylitol_official_plugin_session::plugin::SessionStorePlugin;
use xylitol_official_plugin_tools::plugin::ToolsPlugin;

use crate::surfaces::{PrintPlugin, ServePlugin};

pub fn official_builtins() -> Builtins {
    let mut builtins = Builtins::new();
    builtins.register("session-store", SessionStorePlugin::new);
    builtins.register("compaction", CompactionPlugin::new);
    builtins.register("tools-builtin", ToolsPlugin::new);
    builtins.register("mcp", McpPlugin::new);
    builtins.register("hooks", HooksPlugin::new);
    builtins.register("provider-openai", OpenAiPlugin::new);
    builtins.register("provider-anthropic", AnthropicPlugin::new);
    builtins.register("provider-fake", FakePlugin::new);
    builtins.register("app-print", PrintPlugin::new);
    builtins.register("serve", ServePlugin::new);
    builtins
}
