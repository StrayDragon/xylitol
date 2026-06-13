//! CLI argument parsing and mode dispatch.

use clap::Parser;

use crate::agent::r#loop::AgentLoop;
use crate::agent::model::{ModelConfig, ModelKind};
use crate::agent::registry;
use crate::agent::session::{AgentSession, ModelMeta, ModelRegistry};
use crate::agent::tools::ToolRegistry;
use crate::infra::config::loader::load_app_config;
use crate::infra::session::SessionManager;

#[derive(Parser, Debug)]
#[command(name = "xylitol", version, about)]
pub struct CliArgs {
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

    let cli_config_path = args.config.as_deref().map(std::path::Path::new);

    // ── Step 1: try loading YAML config ──────────────────────────
    let app_config = match load_app_config(cli_config_path) {
        Ok(cfg) => Some(cfg),
        Err(e) => {
            eprintln!("Warning: config load failed ({e}), falling back to env vars");
            None
        }
    };

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
                });
            }
        }
    }

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
        eprintln!(
            "Error: no models configured. Set OPENAI_API_KEY or ANTHROPIC_API_KEY, \
             or create a config.yaml with model entries."
        );
        return Err("no models available".into());
    }

    let tool_registry = ToolRegistry::builtins();
    let sessions_dir = SessionManager::default_dir();
    std::fs::create_dir_all(&sessions_dir).ok();
    let session_mgr = SessionManager::new(sessions_dir);

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

    let mut agent_session = AgentSession::new(
        model_registry,
        tool_registry,
        session_mgr.clone(),
        Some(system_prompt),
        max_iterations,
        0.8,
        cwd,
    );

    // ── Step 4: select model ─────────────────────────────────────
    if let Some(ref mid) = args.model {
        if agent_session.model_registry().find(mid).is_none() {
            eprintln!("Warning: model '{mid}' not found in registry");
        } else {
            let _ = agent_session.select_model(mid);
        }
    }

    let session_id = args
        .session
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let mut agent_loop = AgentLoop::new(agent_session);

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
        #[cfg(feature = "dev-fake-provider")]
        ModelKind::Fake => Some(String::new()),
    }
}
