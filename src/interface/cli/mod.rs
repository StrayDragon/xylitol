//! CLI argument parsing and mode dispatch.

use clap::{Parser, Subcommand};

use crate::agent::auth;
use crate::agent::r#loop::AgentLoop;
use crate::agent::model::registry;
use crate::agent::model::resolver;
use crate::agent::session::{AgentSession, ModelRegistry};
use crate::agent::tools::ToolRegistry;
use crate::core::model::{ModelConfig, ModelKind};
use crate::core::types::ModelMeta;
use crate::infra::config::loader::load_app_config;
use crate::infra::session::SessionManager;
use crate::infra::timing;
use crate::interface::resources::ResourcesAction;

/// Top-level subcommand. When absent, the flat flags/positional below drive
/// the default print-mode flow (backward compatible).
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Read-only resource listing and diagnostics.
    Resources {
        #[command(subcommand)]
        action: ResourcesAction,
    },
    /// Interactive TUI mode (requires `ui-tui` feature).
    Tui,
}

#[derive(Parser, Debug)]
#[command(name = "xylitol", version, about)]
pub struct CliArgs {
    #[command(subcommand)]
    pub command: Option<Command>,

    pub prompt: Option<String>,
    #[arg(long)]
    pub print: bool,
    #[arg(long)]
    pub session: Option<String>,
    #[arg(long)]
    pub model: Option<String>,
    #[arg(long)]
    pub config: Option<String>,
    #[arg(long)]
    pub rpc: bool,
    #[arg(long)]
    pub list_models: bool,
    #[arg(long)]
    pub no_color: bool,
}

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = CliArgs::parse();

    // ── Subcommands requiring model loading (handled below) ────────
    let _is_tui_mode = matches!(args.command, Some(Command::Tui));

    // ── Subcommands: handled early, no model loading needed ─────────
    if let Some(Command::Resources { action }) = args.command {
        let code = crate::interface::resources::run(action);
        if code == std::process::ExitCode::FAILURE {
            std::process::exit(1);
        }
        return Ok(());
    }

    timing::reset_timings();

    let cli_config_path = args.config.as_deref().map(std::path::Path::new);

    // ── Step 1: try loading YAML config ──────────────────────────
    let app_config = match load_app_config(cli_config_path) {
        Ok(cfg) => Some(cfg),
        Err(e) => {
            eprintln!("Warning: config load failed ({e}), falling back to env vars");
            None
        }
    };

    timing::time("config.load");

    // ── Step 2: build ModelRegistry ──────────────────────────────
    let mut model_registry = ModelRegistry::new();

    if let Some(ref cfg) = app_config {
        // YAML models first
        for (alias, entry) in &cfg.model.models {
            let api_key = resolve_api_key(entry.provider);
            if api_key.is_none() {
                continue;
            }

            let context_window = if entry.context_window > 0 {
                entry.context_window
            } else {
                registry::default_context_window_for(entry.provider)
            };

            model_registry.register(ModelMeta {
                id: alias.clone(),
                config: ModelConfig {
                    kind: entry.provider,
                    api_key: api_key.expect("checked above"),
                    model: entry.model.clone(),
                    base_url: entry.base_url.clone(),
                },
                display_name: alias.clone(),
                thinking: entry.thinking,
                context_window,
                api: String::new(),
                provider: String::new(),
                cost_input: 0.0,
                cost_output: 0.0,
                cost_cache_read: 0.0,
                cost_cache_write: 0.0,
                max_tokens: 0,
                thinking_levels: Vec::new(),
            });
        }
    }

    // Fallback: env-var discovery if no models from config
    if model_registry.is_empty() {
        for (provider_name, env_var, kind) in [
            ("openai", "OPENAI_API_KEY", ModelKind::OpenAi),
            ("anthropic", "ANTHROPIC_API_KEY", ModelKind::Anthropic),
        ] {
            if let Ok(key) = std::env::var(env_var)
                && let Some(model_id) = registry::default_model_id_for_provider(provider_name)
            {
                model_registry.register(ModelMeta {
                    id: model_id.to_string(),
                    config: ModelConfig {
                        kind,
                        api_key: key,
                        model: model_id.to_string(),
                        base_url: None,
                    },
                    display_name: model_id.to_string(),
                    thinking: true,
                    context_window: registry::default_context_window_for(kind),
                    api: String::new(),
                    provider: String::new(),
                    cost_input: 0.0,
                    cost_output: 0.0,
                    cost_cache_read: 0.0,
                    cost_cache_write: 0.0,
                    max_tokens: 0,
                    thinking_levels: Vec::new(),
                });
            }
        }
    }

    timing::time("model_registry.load");

    if args.list_models {
        for m in model_registry.list() {
            println!(
                "  {} — {} (ctx: {})",
                m.id, m.display_name, m.context_window
            );
        }
        return Ok(());
    }

    if model_registry.is_empty() {
        eprintln!("Error: {}", auth::format_no_models_available_message());
        return Err("no models available".into());
    }

    let tool_registry = ToolRegistry::builtins();
    let sessions_dir = SessionManager::default_dir();
    std::fs::create_dir_all(&sessions_dir).ok();
    let session_mgr = SessionManager::new(sessions_dir.clone());

    // Check session CWD if restoring a session
    let fallback_cwd = std::env::current_dir().unwrap_or_default();
    if let Some(ref session_arg) = args.session
        && session_mgr.exists(session_arg)
    {
        eprintln!(
            "Info: Restoring session '{}' from {}",
            session_arg,
            sessions_dir.display()
        );
        // Full CWD validation happens on async session load in run_print()
        // If the stored CWD no longer exists, the agent falls back to fallback_cwd.
        let _ = fallback_cwd; // keep for future async validation
    }

    timing::time("session.restore");

    // ── Step 3: determine system prompt ──────────────────────────
    let system_prompt = app_config
        .as_ref()
        .and_then(|cfg| cfg.resolve_default_profile().ok())
        .and_then(|p| p.system_prompt)
        .unwrap_or_else(|| "You are a helpful AI assistant.".into());

    let max_iterations = app_config
        .as_ref()
        .and_then(|cfg| cfg.resolve_default_profile().ok())
        .map(|p| p.max_iterations)
        .unwrap_or(50);

    let cwd = std::env::current_dir()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    // ── Step 3b: discover prompt templates (read-only scan) ─────
    // DefaultResourceLoader scans ~/.xylitol/prompts and <cwd>/.xylitol/prompts.
    // Each becomes a /template:name command after the session is built.
    let discovered_templates = {
        let agent_dir = crate::infra::resource::DefaultResourceLoader::default_agent_dir();
        let loader = crate::infra::resource::DefaultResourceLoader::new(
            std::path::PathBuf::from(&cwd),
            agent_dir,
        );
        loader.get_prompts().0.to_vec()
    };

    let mut agent_session = AgentSession::new(
        model_registry,
        tool_registry,
        session_mgr.clone(),
        Some(system_prompt),
        max_iterations,
        0.8,
        cwd,
    );
    // ── Step 3c: initialize sandbox engine ────────────────────
    if let Some(ref cfg) = app_config
        && let Some(ref sandbox_cfg) = cfg.security.sandbox
        && sandbox_cfg.enabled
    {
        let engine = crate::infra::sandbox::build_engine(sandbox_cfg);
        agent_session.set_sandbox_engine(Some(engine));
    }

    agent_session.register_prompt_commands(&discovered_templates);

    timing::time("session.create");

    // ── Step 4: RPC mode (early return) ────────────────────────
    if args.rpc {
        return crate::interface::rpc::run(agent_session)
            .await
            .map_err(|e| e.into());
    }

    // ── Step 5: select model ─────────────────────────────────────
    if let Some(ref mid) = args.model {
        let available: Vec<&ModelMeta> = agent_session.model_registry().list().iter().collect();
        match resolver::resolve_model(mid, &available, None) {
            Ok(resolved) => {
                if let Some(ref warning) = resolved.warning {
                    eprintln!("Warning: {warning}");
                }
                let _ = agent_session.select_model(&resolved.model.id);
            }
            Err(msg) => {
                eprintln!(
                    "Warning: {}\n{}",
                    msg,
                    auth::format_no_model_selected_message()
                );
            }
        }
    }

    let session_id = args
        .session
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let mut agent_loop = AgentLoop::new(agent_session);

    // ── Step 6: dispatch by mode ─────────────────────────────
    #[cfg(feature = "ui-tui")]
    if _is_tui_mode {
        let model_name = agent_loop
            .session()
            .current_model()
            .map(|m| m.display_name.clone())
            .unwrap_or_else(|| "unknown".to_string());
        crate::interface::tui::run_tui_engine(&mut agent_loop, &session_id, &model_name).await?;
        timing::print_timings();
        return Ok(());
    }

    let prompt = args.prompt.unwrap_or_else(|| {
        use std::io::Read;
        let mut buf = String::new();
        if std::io::stdin().read_to_string(&mut buf).is_ok() && !buf.is_empty() {
            buf.trim().to_string()
        } else {
            "Hello!".into()
        }
    });

    crate::interface::print::run_print(&mut agent_loop, &prompt, &session_id).await?;

    timing::print_timings();
    Ok(())
}

/// Read the API key for a provider from environment variables.
fn resolve_api_key(kind: ModelKind) -> Option<String> {
    match kind {
        ModelKind::OpenAi => std::env::var("OPENAI_API_KEY")
            .or_else(|_| std::env::var("OPENAI_KEY"))
            .ok(),
        ModelKind::Anthropic => std::env::var("ANTHROPIC_API_KEY")
            .or_else(|_| std::env::var("ANTHROPIC_KEY"))
            .ok(),
        ModelKind::Fake => Some(String::new()),
    }
}
