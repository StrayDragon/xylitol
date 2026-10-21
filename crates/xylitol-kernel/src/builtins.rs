use rutis_loader::Builtins;
use xylitol_official_plugin_compaction::plugin::CompactionFactory;
use xylitol_official_plugin_hooks::HooksConfig;
use xylitol_official_plugin_hooks::plugin::HooksFactory;
use xylitol_official_plugin_mcp::plugin::{McpFactory, McpPluginConfig};
use xylitol_official_plugin_provider_anthropic::plugin::AnthropicFactory;
use xylitol_official_plugin_provider_openai::fake_plugin::FakeFactory;
use xylitol_official_plugin_provider_openai::plugin::OpenAiFactory;
use xylitol_official_plugin_session::plugin::{SessionPluginConfig, SessionStoreFactory};
use xylitol_official_plugin_tools::plugin::{ToolsFactory, ToolsPluginConfig};
use xylitol_plugin_sdk::EmptyPluginConfig;
use xylitol_plugin_sdk::compaction_config::XyCompactionSettingsConfig;

use crate::surfaces::{PrintFactory, ServeFactory};

pub fn official_builtins() -> Builtins {
    let mut builtins = Builtins::new();
    builtins.register::<SessionPluginConfig, _>("session-store", SessionStoreFactory);
    builtins.register::<XyCompactionSettingsConfig, _>("compaction", CompactionFactory);
    builtins.register::<ToolsPluginConfig, _>("tools-builtin", ToolsFactory);
    builtins.register::<McpPluginConfig, _>("mcp", McpFactory);
    builtins.register::<HooksConfig, _>("hooks", HooksFactory);
    builtins.register::<EmptyPluginConfig, _>("provider-openai", OpenAiFactory);
    builtins.register::<EmptyPluginConfig, _>("provider-anthropic", AnthropicFactory);
    builtins.register::<EmptyPluginConfig, _>("provider-fake", FakeFactory);
    builtins.register::<EmptyPluginConfig, _>("app-print", PrintFactory);
    builtins.register::<EmptyPluginConfig, _>("serve", ServeFactory);
    builtins
}
