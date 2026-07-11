//! CLI argument parsing and mode dispatch.
//!
//! Provider-guidance messages (login help, no-model/no-api-key text) live in
//! [`provider_guidance`] — they are pure CLI-surface presentation, not agent
//! orchestration (la13).

mod logging;
mod print;
mod provider_guidance;
pub mod resources;

use clap::{Parser, Subcommand};

use crate::app::cli::resources::ResourcesAction;
use crate::app::core::bootstrap::{
    BootstrapError, BootstrapInput, BootstrapWarning, BootstrappedAgent, bootstrap,
    resolve_assembly,
};
use crate::app::core::driver::InProcessDriver;
#[cfg(feature = "server")]
use crate::app::server::subcommand::ServerSubcommand;
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
    /// Enter the interactive inline TUI (ratatui). Implied when no prompt is
    /// given and stdin is a TTY.
    #[arg(long)]
    pub tui: bool,
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

    // ── Install the tracing subscriber (env-driven, file-only) ──────
    // Done before any mode dispatch so every surface (print / TUI / RPC /
    // subcommands) is covered. Debug builds default on; release needs
    // RUST_LOG / XYLITOL_DEBUG. File-only (~/.xylitol/logs/xylitol.log) —
    // never stdout/stderr (see app/cli/logging.rs).
    logging::init_logging(&crate::infra::resource::DefaultResourceLoader::default_agent_dir());

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

    let trust_override = match (args.trust, args.no_trust) {
        (true, _) => Some(true),
        (_, true) => Some(false),
        _ => None,
    };
    let bootstrap_input = BootstrapInput {
        config_path: args.config.as_ref().map(std::path::PathBuf::from),
        session: args.session.clone(),
        model: args.model.clone(),
        trust_override,
        interactive: false, // print mode has no interactive trust UI
        caller: "cli",
    };

    // `--list-models` needs the resolved registry before any agent build; it
    // resolves assembly (cheap — no agent construction) and returns early.
    if args.list_models {
        let assembly = match resolve_assembly(&bootstrap_input) {
            Ok(a) => a,
            Err(BootstrapError::NoModelsAvailable) => {
                eprintln!(
                    "Error: {}",
                    provider_guidance::format_no_models_available_message()
                );
                return Err("no models available".into());
            }
            Err(e) => return Err(e.into()),
        };
        render_warnings(&assembly.warnings);
        for m in assembly.model_registry.list() {
            println!(
                "  {} — {} (ctx: {})",
                m.id, m.display_name, m.context_window
            );
        }
        return Ok(());
    }

    // ── print / tui: build once via bootstrap ────────────────────
    let bootstrapped = match bootstrap(bootstrap_input) {
        Ok(b) => b,
        Err(BootstrapError::NoModelsAvailable) => {
            eprintln!(
                "Error: {}",
                provider_guidance::format_no_models_available_message()
            );
            return Err("no models available".into());
        }
        Err(e) => return Err(e.into()),
    };
    render_warnings(&bootstrapped.warnings);
    let BootstrappedAgent {
        agent,
        session_id,
        store,
        mcp_servers,
        ..
    } = bootstrapped;
    let mut driver = InProcessDriver::new(agent, store);
    let mut mcp = crate::app::core::composition::McpSession::new();
    let servers = mcp_servers.unwrap_or_default();
    if let Err(e) = mcp.reload(&mut driver, &servers).await {
        eprintln!("Warning: MCP reload failed: {e}");
    }
    // `mcp` kept for process lifetime (owns MCP connections when enabled).
    let _mcp = mcp;

    // ── dispatch by mode ───────────────────────────────────────
    // TUI: when no prompt is supplied and stdin is a TTY (mirroring pi's
    // resolveAppMode), enter the inline REPL. `--tui` forces it even with a
    // prompt. Rpc was handled earlier above.
    #[cfg(feature = "tui")]
    {
        use std::io::IsTerminal;
        let want_tui = args.tui || (args.prompt.is_none() && std::io::stdin().is_terminal());
        if want_tui {
            return crate::app::tui::run(&mut driver)
                .await
                .map_err(|e| e.into());
        }
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

    crate::app::cli::print::run_print(&mut driver, &prompt, &session_id).await?;

    timing::print_timings();
    Ok(())
}

/// Render bootstrap diagnostics as CLI warnings (print-mode byte-compat).
///
/// `provider_guidance` enriches the bare warning text (login help etc.) — this
/// is the single place where structured [`BootstrapWarning`]s become user-facing
/// CLI text, keeping `app::core::bootstrap` free of presentation deps.
fn render_warnings(warnings: &[BootstrapWarning]) {
    for w in warnings {
        match w {
            BootstrapWarning::ConfigLoadFailed(e) => {
                eprintln!("Warning: config load failed ({e}), falling back to env vars");
            }
            BootstrapWarning::ConfigLoadedZeroModels => {
                eprintln!(
                    "Warning: config file present but loaded 0 models. \
                     A structural typo such as `model:` (singular) instead of `models:` \
                     (plural) is silently ignored. See configs/example.yaml."
                );
            }
            BootstrapWarning::NoApiKey { provider } => {
                eprintln!(
                    "Warning: {}",
                    provider_guidance::format_no_api_key_found_message(provider)
                );
            }
            BootstrapWarning::ProjectNotTrusted { reason } => {
                eprintln!(
                    "Project not trusted ({reason}); skipping `.xylitol/` project resources."
                );
            }
            BootstrapWarning::RestoringSession { session, path } => {
                eprintln!("Info: Restoring session '{session}' from {path}");
            }
            BootstrapWarning::ModelResolutionWarning(warning) => {
                eprintln!("Warning: {warning}");
            }
            BootstrapWarning::ModelResolutionFailed(msg) => {
                eprintln!(
                    "Warning: {msg}\n{}",
                    provider_guidance::format_no_model_selected_message()
                );
            }
        }
    }
}
