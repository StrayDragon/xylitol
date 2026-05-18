//! CLI argument parsing and mode dispatch.
//!
//! Mode is auto-detected from positional args and flags:
//! - prompt present → print/stdio mode
//! - no prompt → interactive/TUI mode (requires `ui-tui` feature)
//! - `--acp` flag → ACP mode (requires `infra-acp` feature)

use std::sync::Arc;

use adk_session::InMemorySessionService;
use clap::Parser;

use crate::agent::tools::ToolRegistry;
use crate::infra::config;
use crate::infra::config::types::ProviderKind;
use crate::interface::print;

/// xylitol — LLM-Augmented Development Toolkit
#[derive(Parser, Debug)]
#[command(name = "xylitol", version, about)]
pub(crate) struct CliArgs {
    /// Prompt to process (omitting it starts interactive mode)
    pub(crate) prompt: Option<String>,

    /// Activate ACP mode (IDE integration via stdio)
    #[cfg(feature = "infra-acp")]
    #[arg(long)]
    pub(crate) acp: bool,

    /// List available models and exit
    #[arg(long)]
    pub(crate) list_models: bool,

    /// Path to config file
    #[arg(long)]
    pub(crate) config: Option<String>,

    /// Project root directory
    #[arg(long)]
    pub(crate) project: Option<String>,

    /// Override default model (use __fake__ for dev fake provider)
    #[arg(long)]
    pub(crate) model: Option<String>,

    /// Disable ANSI colour output
    #[arg(long)]
    pub(crate) no_color: bool,

    /// Skip all confirmations
    #[arg(long)]
    pub(crate) yolo: bool,
}

/// Entry point: parse args → load config → dispatch by auto-detected mode.
pub(crate) fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = CliArgs::parse();

    let config_path = args.config.as_deref().map(std::path::Path::new);
    let app_config = config::load_app_config(config_path)?;

    // Early-exit: list models and quit.
    if args.list_models {
        return list_models_and_exit(&app_config);
    }

    // Override project root if --project was provided
    if let Some(ref project) = args.project {
        tracing::info!("project root: {project}");
    }

    // Override model if --model was provided
    if let Some(ref model) = args.model {
        tracing::info!("model override: {model}");
    }

    if args.no_color {
        tracing::info!("ANSI colour output disabled");
    }

    // ACP mode: explicit opt-in via flag.
    #[cfg(feature = "infra-acp")]
    if args.acp {
        tracing::info!("ACP mode — implemented in c87-add-acp-mode");
        return Ok(());
    }

    // Auto-detect mode from positional prompt.
    match args.prompt {
        Some(ref prompt) if !prompt.is_empty() => {
            tracing::info!("print/stdio mode");
            run_print_mode(&args, &app_config, prompt)
        }
        _ => {
            #[cfg(feature = "ui-tui")]
            {
                tracing::info!("interactive/TUI mode");
                tracing::info!("TUI mode — implemented in c80-add-tui");
                Ok(())
            }
            #[cfg(not(feature = "ui-tui"))]
            {
                Err(
                    "no prompt provided and interactive mode is not available (ui-tui feature not enabled) \
                     — use: xylitol \"your prompt\""
                        .into(),
                )
            }
        }
    }
}

/// Run print/stdio mode with the given prompt.
fn run_print_mode(
    args: &CliArgs,
    app_config: &config::AppConfig,
    prompt: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Runtime::new()?;
    #[allow(unused_mut)]
    let mut tools = ToolRegistry::builtins();

    // Register MCP tools from configured servers.
    #[cfg(feature = "infra-skills")]
    rt.block_on(register_mcp_tools(&mut tools, app_config))?;

    let profile = build_resolved_profile(app_config, args.model.as_deref())?;
    let session_service = Arc::new(InMemorySessionService::new());

    rt.block_on(print::run_print(
        prompt,
        &tools,
        app_config,
        &profile,
        session_service,
        args.no_color,
    ))?;

    Ok(())
}

/// Connect to all configured MCP servers and register their tools into the registry.
#[cfg(feature = "infra-skills")]
async fn register_mcp_tools(
    tools: &mut ToolRegistry,
    app_config: &config::AppConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::infra::skills::{McpClientManager, McpToolAdapter};
    use std::sync::Arc;

    let manager = Arc::new(McpClientManager::new());
    manager
        .connect(app_config)
        .await
        .map_err(|e| format!("MCP connect: {e}"))?;
    let mcp_tools = manager.list_all_tools().await;
    for (server_id, tool_name, description, schema) in mcp_tools {
        let adapter = McpToolAdapter::new(
            server_id,
            tool_name,
            description,
            Some(schema),
            manager.clone(),
        );
        tools.register(Arc::new(adapter));
    }
    Ok(())
}

/// Print available models from config and exit.
fn list_models_and_exit(config: &config::AppConfig) -> Result<(), Box<dyn std::error::Error>> {
    let default = &config.model.default_model;
    println!("Available models (default: {default}):\n");

    if config.model.models.is_empty() {
        println!("  (no models configured)");
    } else {
        // Header
        println!("  {:<20} {:<12} MODEL", "ALIAS", "PROVIDER");
        for (alias, entry) in &config.model.models {
            let default_marker = if alias == default { " *" } else { "" };
            let provider = match entry.provider {
                ProviderKind::OpenAI => "openai",
                ProviderKind::Anthropic => "anthropic",
            };
            println!(
                "  {:<20} {:<12} {}{default_marker}",
                alias, provider, entry.model
            );
        }
    }

    #[cfg(feature = "dev-fake-provider")]
    {
        println!("\nSpecial models:");
        println!(
            "  {:<20} {:<12} {}",
            "__fake__", "fake", "scenario-based [dev]"
        );
    }

    Ok(())
}

/// Build a [`ResolvedProfile`] from app config, optionally overridden by `--model`.
fn build_resolved_profile(
    config: &config::AppConfig,
    model_override: Option<&str>,
) -> Result<crate::agent::profile::ResolvedProfile, Box<dyn std::error::Error>> {
    // --- Fake provider short-circuit (CLI-specific) ---
    #[cfg(feature = "dev-fake-provider")]
    if model_override == Some("__fake__") {
        use crate::agent::model::{ModelConfig, ModelKind};
        return Ok(crate::agent::profile::ResolvedProfile {
            model_config: ModelConfig {
                kind: ModelKind::Fake,
                api_key: String::new(),
                model: "__fake__".into(),
                base_url: None,
            },
            system_prompt: None,
            allowed_tools: None,
            max_iterations: 50,
            name: "__fake__".into(),
        });
    }
    #[cfg(not(feature = "dev-fake-provider"))]
    if model_override == Some("__fake__") {
        return Err(
            "the __fake__ model requires the dev-fake-provider feature flag — \
             rebuild with: cargo run --features dev-fake-provider"
                .into(),
        );
    }

    let mut profile = config.resolve_default_profile()?;

    // --model overrides the profile's model.
    if let Some(model_id) = model_override {
        profile.model_config = config.resolve_model(model_id)?;
    }

    Ok(profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_argument() {
        let args = CliArgs::parse_from(["xylitol", "my prompt here"]);
        assert_eq!(args.prompt.as_deref(), Some("my prompt here"));
    }

    #[test]
    fn test_no_prompt() {
        let args = CliArgs::parse_from(["xylitol"]);
        assert!(args.prompt.is_none());
    }

    #[test]
    fn test_list_models_flag() {
        let args = CliArgs::parse_from(["xylitol", "--list-models"]);
        assert!(args.list_models);
    }

    #[test]
    fn test_list_models_default_false() {
        let args = CliArgs::parse_from(["xylitol"]);
        assert!(!args.list_models);
    }

    #[test]
    fn test_config_path() {
        let args = CliArgs::parse_from(["xylitol", "--config", "/tmp/test.yaml"]);
        assert_eq!(args.config.as_deref(), Some("/tmp/test.yaml"));
    }

    #[test]
    fn test_project_path() {
        let args = CliArgs::parse_from(["xylitol", "--project", "/my/project"]);
        assert_eq!(args.project.as_deref(), Some("/my/project"));
    }

    #[test]
    fn test_model_override() {
        let args = CliArgs::parse_from(["xylitol", "--model", "claude-opus-4"]);
        assert_eq!(args.model.as_deref(), Some("claude-opus-4"));
    }

    #[test]
    fn test_model_fake_sentinel() {
        let args = CliArgs::parse_from(["xylitol", "--model", "__fake__"]);
        assert_eq!(args.model.as_deref(), Some("__fake__"));
    }

    #[test]
    fn test_yolo_flag() {
        let args = CliArgs::parse_from(["xylitol", "--yolo"]);
        assert!(args.yolo);
    }

    #[test]
    fn test_yolo_default_false() {
        let args = CliArgs::parse_from(["xylitol"]);
        assert!(!args.yolo);
    }

    #[test]
    fn test_all_options() {
        let args = CliArgs::parse_from([
            "xylitol",
            "--config",
            "/tmp/c.yaml",
            "--project",
            "/proj",
            "--model",
            "gpt-4o",
            "--yolo",
            "--list-models",
            "some prompt",
        ]);
        assert_eq!(args.config.as_deref(), Some("/tmp/c.yaml"));
        assert_eq!(args.project.as_deref(), Some("/proj"));
        assert_eq!(args.model.as_deref(), Some("gpt-4o"));
        assert!(args.yolo);
        assert!(args.list_models);
        assert_eq!(args.prompt.as_deref(), Some("some prompt"));
    }

    #[test]
    fn test_auto_detect_mode_with_prompt() {
        let args = CliArgs::parse_from(["xylitol", "do something"]);
        assert!(args.prompt.is_some());
        assert!(!args.list_models);
    }

    #[test]
    fn test_auto_detect_mode_without_prompt() {
        let args = CliArgs::parse_from(["xylitol"]);
        assert!(args.prompt.is_none());
    }
}
