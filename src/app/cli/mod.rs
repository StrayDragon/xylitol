//! CLI argument parsing and mode dispatch.
//!
//! Provider-guidance messages (login help, no-model/no-api-key text) live in
//! [`provider_guidance`] — they are pure CLI-surface presentation, not agent
//! orchestration (la13).

mod print;
mod provider_guidance;
pub mod resources;

use std::sync::Arc;

use clap::{Parser, Subcommand};

use crate::agent::model::registry;
use crate::agent::model::resolver;
use crate::agent::session::ModelRegistry;
use crate::app::cli::resources::ResourcesAction;
use crate::app::core::composition::{BuildAgentOptions, build_agent};
use crate::app::core::driver::InProcessDriver;
#[cfg(feature = "server")]
use crate::app::server::subcommand::ServerSubcommand;
use crate::domain::model::{XyModelConfig, XyModelKind};
use crate::domain::types::XyModelMeta;
use crate::infra::config::loader::load_app_config;
use crate::infra::config::value::InfraSecretResolver;
use crate::infra::session::SessionManager;
use crate::infra::timing;

/// Top-level subcommand. When absent, the flat flags/positional below drive
/// the default print-mode flow (backward compatible).
#[derive(Subcommand, Debug)]
pub enum CliCommand {
    /// Read-only resource listing and diagnostics.
    Resources {
        #[command(subcommand)]
        action: ResourcesAction,
    },
    /// Server lifecycle management.
    #[cfg(feature = "server")]
    Server {
        #[command(subcommand)]
        action: ServerSubcommand,
    },
}

#[derive(Parser, Debug)]
#[command(name = "xylitol", version, about)]
pub struct CliArgs {
    #[command(subcommand)]
    pub command: Option<CliCommand>,

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
    /// Trust the project directory and load its `.xylitol/` resources.
    #[arg(long)]
    pub trust: bool,
    /// Do not trust the project directory; skip its `.xylitol/` resources.
    #[arg(long)]
    pub no_trust: bool,
}

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = CliArgs::parse();

    // ── Subcommands: handled early, no model loading needed ─────────
    match args.command {
        Some(CliCommand::Resources { action }) => {
            let code = crate::app::cli::resources::run(action);
            if code == std::process::ExitCode::FAILURE {
                std::process::exit(1);
            }
            return Ok(());
        }
        #[cfg(feature = "server")]
        Some(CliCommand::Server { action }) => {
            return crate::app::server::subcommand::run(action).await;
        }
        None => {} // continue to default print-mode flow
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

    // Track whether a config file was successfully loaded. Used later to detect
    // the "config present but zero models loaded" case (likely a typo like
    // `model:` instead of `models:`) and report it instead of silently falling
    // back to environment-variable defaults.
    let config_loaded = app_config.is_some();

    // ── Step 2: build ModelRegistry ──────────────────────────────
    let secret_resolver: Arc<dyn crate::runtime_protocol::XySecretResolver> =
        Arc::new(InfraSecretResolver::new());
    let mut model_registry = ModelRegistry::new(secret_resolver);

    if let Some(ref cfg) = app_config {
        // YAML models first
        for (alias, entry) in &cfg.model.models {
            let api_key = resolve_api_key(entry.provider);
            if api_key.is_none() {
                eprintln!(
                    "Warning: {}",
                    provider_guidance::format_no_api_key_found_message(
                        entry.provider.provider_name()
                    )
                );
                continue;
            }

            let context_window = if entry.context_window > 0 {
                entry.context_window
            } else {
                registry::default_context_window_for(entry.provider)
            };

            model_registry.register(XyModelMeta {
                id: alias.clone(),
                config: XyModelConfig {
                    kind: entry.provider,
                    api_key: api_key.expect("checked above"),
                    model: entry.model.clone(),
                    base_url: entry.base_url.clone(),
                    api: None,
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

    if config_loaded && model_registry.is_empty() {
        // A config file was loaded but contributed zero models. The most common
        // cause is a structural typo (e.g. `model:` instead of `models:`),
        // which serde silently drops under `#[serde(default)]`). Warn loudly so
        // the user knows their config did not apply; do not abort, since the
        // environment-variable discovery below may still rescue the run.
        eprintln!(
            "Warning: config file present but loaded 0 models. \
             A structural typo such as `model:` (singular) instead of `models:` \
             (plural) is silently ignored. See configs/example.yaml."
        );
    }

    // Fallback: env-var discovery if no models from config
    if model_registry.is_empty() {
        for (provider_name, env_var, kind) in [
            ("openai", "OPENAI_API_KEY", XyModelKind::OpenAi),
            ("anthropic", "ANTHROPIC_API_KEY", XyModelKind::Anthropic),
        ] {
            if let Ok(key) = std::env::var(env_var)
                && let Some(model_id) = registry::default_model_id_for_provider(provider_name)
            {
                model_registry.register(XyModelMeta {
                    id: model_id.to_string(),
                    config: XyModelConfig {
                        kind,
                        api_key: key,
                        model: model_id.to_string(),
                        base_url: None,
                        api: None,
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
        eprintln!(
            "Error: {}",
            provider_guidance::format_no_models_available_message()
        );
        return Err("no models available".into());
    }

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
    // Resolve the default profile once; it drives system_prompt, max_iterations,
    // and the startup model (Step 5). Avoids resolving repeatedly below.
    let resolved_profile = app_config
        .as_ref()
        .and_then(|cfg| cfg.resolve_default_profile().ok());
    // System prompt precedence: loader's SYSTEM.md (from resource discovery
    // below) > config profile's system_prompt > None (let build_system_prompt
    // supply its built-in default). The hardcoded "You are a helpful AI
    // assistant." fallback was removed in c280 in favor of the loader path.
    let config_system_prompt = resolved_profile
        .as_ref()
        .and_then(|p| p.system_prompt.clone());

    let max_iterations = resolved_profile
        .as_ref()
        .map(|p| p.max_iterations)
        .unwrap_or(50);

    let cwd = std::env::current_dir()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    // ── Step 3b: resolve project trust and discover resources ─────
    //
    // The project's `.xylitol/` resources (prompts, skills, settings) are only
    // loaded when the project CWD is trusted (spec c255 / t5). Trust is decided
    // here via the resolution pipeline (t3): CLI override → store → default.
    // There is no interactive REPL yet, so the UI prompt callback is a no-op
    // that denies (t4 wiring point for a future interactive loop).
    let trust_override = match (args.trust, args.no_trust) {
        (true, _) => Some(true),
        (_, true) => Some(false),
        _ => None,
    };
    let trust_manager =
        crate::infra::trust::TrustManager::new(crate::infra::trust::TrustManager::default_dir());
    let trust_resolution = crate::infra::trust::resolve_project_trusted(
        &trust_manager,
        &cwd,
        trust_override,
        crate::infra::trust::DefaultProjectTrust::default(),
        false, // no interactive UI in print mode
        |_| None,
    );
    let project_trusted = trust_resolution.trusted;
    if !project_trusted {
        eprintln!(
            "Project not trusted ({}); skipping `.xylitol/` project resources.",
            match trust_resolution.reason {
                crate::infra::trust::TrustReason::Override => "overridden via --no-trust",
                crate::infra::trust::TrustReason::Store => "denied in trust store",
                crate::infra::trust::TrustReason::FallbackNoUi => "no interactive prompt available",
                _ => "policy",
            }
        );
    }

    // DefaultResourceLoader scans ~/.xylitol/prompts and <cwd>/.xylitol/prompts.
    // Each becomes a /template:name command after the session is built.
    // When untrusted, point cwd at a throwaway dir so no project `.xylitol/`
    // resources are discovered (spec t5 gate).
    //
    // The same loader also supplies the system-prompt context (AGENTS.md /
    // CLAUDE.md context files, SYSTEM.md, APPEND_SYSTEM.md) wired into the
    // agent's prompt_opts (c280).
    let (discovered_templates, context_files, loader_system_prompt, append_system_prompt) = {
        let agent_dir = crate::infra::resource::DefaultResourceLoader::default_agent_dir();
        let loader_cwd = if project_trusted {
            std::path::PathBuf::from(&cwd)
        } else {
            // An empty dir cannot contain `.xylitol/`, so project resources
            // are skipped while global resources still load.
            std::env::temp_dir()
        };
        let loader = crate::infra::resource::DefaultResourceLoader::new(loader_cwd, agent_dir);
        let templates = loader.get_prompts().0.to_vec();
        let ctx: Vec<(String, String)> = loader
            .get_agents_files()
            .iter()
            .map(|f| (f.path.to_string_lossy().into_owned(), f.content.clone()))
            .collect();
        let sys = loader.get_system_prompt().map(String::from);
        let append = loader.get_append_system_prompt().to_vec();
        (templates, ctx, sys, append)
    };

    // ── Step 3b2: load compaction settings (settings.json) ───────
    //
    // The agent's compaction tuning (reserve / keep-recent tokens) is read
    // from the merged settings.json (global + project). When no compaction
    // block is configured, pass `None` so the orchestrator uses its defaults.
    let compaction_settings = {
        let agent_dir = crate::infra::resource::DefaultResourceLoader::default_agent_dir();
        let settings_cwd = if project_trusted {
            std::path::PathBuf::from(&cwd)
        } else {
            std::env::temp_dir()
        };
        let settings_mgr = crate::infra::settings::SettingsManager::from_files(
            &settings_cwd,
            &agent_dir,
            project_trusted,
        );
        settings_mgr
            .get_settings()
            .compaction
            .as_ref()
            .map(|c| crate::agent::compaction::CompactionSettings::from(c.clone()))
    };

    // ── Step 3c: initialize permission engine ────────────────────
    let permission = app_config.as_ref().and_then(|cfg| {
        cfg.security
            .permission
            .as_ref()
            .filter(|sc| sc.enabled)
            .map(|sc| crate::infra::permission::build_permission(sc))
    });

    // ── Step 3c1: merge system-prompt sources (loader SYSTEM.md > config) ──
    let system_prompt = loader_system_prompt.or(config_system_prompt);

    // ── Step 4: RPC mode ─────────────────────────────────────────
    if args.rpc {
        return crate::app::rpc::run(
            model_registry,
            session_mgr,
            system_prompt,
            context_files,
            append_system_prompt,
            max_iterations,
            0.8,
            cwd,
            compaction_settings,
            args.session,
            permission,
        )
        .await
        .map_err(|e| e.into());
    }

    // ── Step 5: construct Agent via composition root helper ────
    let mut agent = build_agent(BuildAgentOptions {
        model_registry,
        system_prompt,
        context_files,
        append_system_prompt,
        max_iterations,
        compaction_threshold: 0.8,
        cwd,
        compaction_settings,
        permission,
    })?;

    agent
        .inner_mut()
        .register_prompt_commands(&discovered_templates);

    timing::time("session.create");

    // ── Step 6: select model ────────────────────────────────
    // Priority: explicit `--model` flag > resolved default profile's model
    // (agents.profiles.<default>.model > execution.model > models.default_model).
    let target_model: Option<String> = args.model.clone().or_else(|| {
        resolved_profile
            .as_ref()
            .map(|p| p.model_config.model.clone())
    });

    if let Some(mid) = target_model {
        let available: Vec<&XyModelMeta> = agent.inner().model_registry().list().iter().collect();
        match resolver::resolve_model(&mid, &available, None) {
            Ok(resolved) => {
                if let Some(ref warning) = resolved.warning {
                    eprintln!("Warning: {warning}");
                }
                let _ = agent.inner_mut().select_model(&resolved.model.id);
            }
            Err(msg) => {
                eprintln!(
                    "Warning: {}\n{}",
                    msg,
                    provider_guidance::format_no_model_selected_message()
                );
            }
        }
    }

    let session_id = args
        .session
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let mut driver = InProcessDriver::new(agent);

    // ── Step 6: dispatch by mode ─────────────────────────────
    let prompt = args.prompt.unwrap_or_else(|| {
        use std::io::Read;
        let mut buf = String::new();
        if std::io::stdin().read_to_string(&mut buf).is_ok() && !buf.is_empty() {
            buf.trim().to_string()
        } else {
            "Hello!".into()
        }
    });

    crate::app::cli::print::run_print(&mut driver, &prompt, &session_id).await?;

    timing::print_timings();
    Ok(())
}

/// Read the API key for a provider from environment variables.
fn resolve_api_key(kind: XyModelKind) -> Option<String> {
    match kind {
        XyModelKind::OpenAi => std::env::var("OPENAI_API_KEY")
            .or_else(|_| std::env::var("OPENAI_KEY"))
            .ok(),
        XyModelKind::Anthropic => std::env::var("ANTHROPIC_API_KEY")
            .or_else(|_| std::env::var("ANTHROPIC_KEY"))
            .ok(),
        XyModelKind::Fake => Some(String::new()),
    }
}
