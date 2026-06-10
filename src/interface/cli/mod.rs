//! CLI argument parsing and mode dispatch.

use clap::Parser;

use crate::agent::r#loop::AgentLoop;
use crate::agent::model::{ModelConfig, ModelKind};
use crate::agent::session::{AgentSession, ModelMeta, ModelRegistry};
use crate::agent::tools::ToolRegistry;
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
    pub rpc: bool,
    #[arg(long)]
    pub list_models: bool,
    #[arg(long)]
    pub no_color: bool,
}

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = CliArgs::parse();

    let mut model_registry = ModelRegistry::new();

    if let Ok(key) = std::env::var("OPENAI_API_KEY") {
        model_registry.register(ModelMeta {
            id: "gpt-4o".into(),
            config: ModelConfig {
                kind: ModelKind::OpenAi,
                api_key: key,
                model: "gpt-4o".into(),
                base_url: None,
            },
            display_name: "GPT-4o".into(),
            thinking: false,
            context_window: 128000,
        });
    }
    if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
        model_registry.register(ModelMeta {
            id: "claude-sonnet".into(),
            config: ModelConfig {
                kind: ModelKind::Anthropic,
                api_key: key,
                model: "claude-sonnet-4-20250514".into(),
                base_url: None,
            },
            display_name: "Claude Sonnet 4".into(),
            thinking: true,
            context_window: 200000,
        });
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

    let tool_registry = ToolRegistry::builtins();
    let sessions_dir = SessionManager::default_dir();
    std::fs::create_dir_all(&sessions_dir).ok();
    let session_mgr = SessionManager::new(sessions_dir);

    let mut agent_session = AgentSession::new(
        model_registry,
        tool_registry,
        session_mgr.clone(),
        Some("You are a helpful AI assistant.".into()),
        50,
        0.8,
        std::env::current_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
    );
    if let Some(ref mid) = args.model {
        if agent_session.model_registry().find(mid).is_none() {
            eprintln!("Warning: model '{mid}' not found");
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
