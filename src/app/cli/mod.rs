//! CLI argument parsing and mode dispatch.
//!
//! Provider-guidance messages (login help, no-model/no-api-key text) live in
//! the CLI provider-guidance helpers — they are pure CLI-surface presentation, not agent
//! orchestration (la13).

mod logging;
mod print;
mod provider_guidance;
pub mod resources;
pub mod tokenizer;
#[cfg(feature = "tui")]
mod trust_gate;

use clap::{Parser, Subcommand};

use crate::app::cli::resources::ResourcesAction;
use crate::app::cli::tokenizer::TokenizerAction;
use crate::app::core::bootstrap::{
    BootstrapError, BootstrapInput, BootstrapWarning, bootstrap, resolve_assembly,
};
#[cfg(feature = "server")]
use crate::app::server::subcommand::ServerSubcommand;
use crate::infra::timing;

/// Top-level subcommand. When absent, flat flags/positional drive TUI (default)
/// or print one-shot.
#[derive(Subcommand, Debug)]
pub enum CliCommand {
    /// Read-only resource listing and diagnostics.
    Resources {
        #[command(subcommand)]
        action: ResourcesAction,
    },
    /// Local tokenizer cache: status / opt-in download / clean (c1380).
    Tokenizer {
        #[command(subcommand)]
        action: TokenizerAction,
    },
    /// Server lifecycle management.
    #[cfg(feature = "server")]
    Server {
        #[command(subcommand)]
        action: ServerSubcommand,
    },
}

#[derive(Parser, Debug)]
#[command(
    name = "xylitol",
    version,
    about = "Personal coding agent — default entry is the interactive TUI"
)]
pub struct CliArgs {
    #[command(subcommand)]
    pub command: Option<CliCommand>,

    /// One-shot prompt for print mode (`--prompt` / `-p`).
    #[arg(short = 'p', long = "prompt", value_name = "TEXT")]
    pub prompt_flag: Option<String>,

    /// Positional one-shot prompt (same as `--prompt`).
    #[arg(value_name = "PROMPT")]
    pub positional_prompt: Option<String>,

    /// Force print mode (still requires a prompt via `--prompt`, positional, or piped stdin).
    #[arg(long)]
    pub print: bool,

    #[arg(long)]
    pub session: Option<String>,
    #[arg(long)]
    pub model: Option<String>,
    #[arg(long)]
    pub config: Option<String>,

    /// Force the interactive TUI (default when no one-shot prompt on a TTY).
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

impl CliArgs {
    /// Merged one-shot prompt: `--prompt` wins over positional.
    pub fn one_shot_prompt(&self) -> Option<&str> {
        self.prompt_flag
            .as_deref()
            .or(self.positional_prompt.as_deref())
            .map(str::trim)
            .filter(|s| !s.is_empty())
    }
}

/// Which interactive surface to open (after `--list-models` is ruled out).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceMode {
    Tui,
    Print,
}

/// Pure dispatch (c474): TUI is the default on a TTY; print needs an explicit prompt path.
pub fn select_surface_mode(
    force_tui: bool,
    print_flag: bool,
    has_one_shot_prompt: bool,
    stdin_is_tty: bool,
) -> SurfaceMode {
    if force_tui {
        return SurfaceMode::Tui;
    }
    if has_one_shot_prompt || print_flag {
        return SurfaceMode::Print;
    }
    if stdin_is_tty {
        return SurfaceMode::Tui;
    }
    // Non-TTY bare launch: treat as print so the caller can error (no Hello!).
    SurfaceMode::Print
}

/// Resolve the print-mode prompt. Never returns a placeholder like `Hello!`.
pub fn resolve_print_prompt(
    one_shot: Option<&str>,
    allow_stdin_pipe: bool,
    stdin_is_tty: bool,
    mut read_stdin: impl FnMut() -> std::io::Result<String>,
) -> Result<String, String> {
    if let Some(p) = one_shot.map(str::trim).filter(|s| !s.is_empty()) {
        return Ok(p.to_string());
    }
    if allow_stdin_pipe && !stdin_is_tty {
        let buf = read_stdin().map_err(|e| format!("failed to read stdin: {e}"))?;
        let trimmed = buf.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }
    Err(
        "print mode requires a prompt: pass PROMPT, --prompt TEXT, or pipe stdin \
         (bare launch on a TTY opens the TUI; use --tui to force it)"
            .into(),
    )
}

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = CliArgs::parse();

    // ── Install file-only observability (fastrace + log) ────────────
    // Done before any mode dispatch so every surface (print / TUI / RPC /
    // subcommands) is covered. Debug builds default on; release needs
    // RUST_LOG / XYLITOL_DEBUG / XYLITOL_PROVIDER_TRACE. Never stdout/stderr.
    logging::init_logging(&crate::infra::resource::DefaultResourceLoader::default_agent_dir());
    struct FlushOnDrop;
    impl Drop for FlushOnDrop {
        fn drop(&mut self) {
            logging::flush_observability();
        }
    }
    let _flush = FlushOnDrop;

    // ── Subcommands: handled early, no model loading needed ─────────
    match args.command {
        Some(CliCommand::Resources { action }) => {
            let code = crate::app::cli::resources::run(action);
            if code == std::process::ExitCode::FAILURE {
                std::process::exit(1);
            }
            return Ok(());
        }
        Some(CliCommand::Tokenizer { action }) => {
            let code = crate::app::cli::tokenizer::run(action).await;
            if code == std::process::ExitCode::FAILURE {
                std::process::exit(1);
            }
            return Ok(());
        }
        #[cfg(feature = "server")]
        Some(CliCommand::Server { action }) => {
            return crate::app::server::subcommand::run(action).await;
        }
        None => {}
    }

    timing::reset_timings();

    let trust_override = match (args.trust, args.no_trust) {
        (true, _) => Some(true),
        (_, true) => Some(false),
        _ => None,
    };

    use std::io::IsTerminal;
    let stdin_is_tty = std::io::stdin().is_terminal();
    let one_shot = args.one_shot_prompt().map(str::to_string);

    #[cfg(feature = "tui")]
    let want_tui = !args.list_models
        && select_surface_mode(args.tui, args.print, one_shot.is_some(), stdin_is_tty)
            == SurfaceMode::Tui;
    #[cfg(not(feature = "tui"))]
    let want_tui = false;

    // c490: Ask trust inside ChoicePrompt **before** bootstrap (no stdio menu).
    #[cfg(feature = "tui")]
    if want_tui {
        match trust_gate::run_trust_gate_if_needed(trust_override) {
            Ok(()) => {}
            Err(trust_gate::TrustGateError::Cancelled) => {
                eprintln!("{}", trust_gate::TrustGateError::Cancelled);
                return Ok(());
            }
            Err(trust_gate::TrustGateError::Denied) => {
                eprintln!("{}", trust_gate::TrustGateError::Denied);
                return Ok(());
            }
            Err(e) => return Err(e.to_string().into()),
        }
    }

    let bootstrap_input = BootstrapInput {
        config_path: args.config.as_ref().map(std::path::PathBuf::from),
        session: args.session.clone(),
        model: args.model.clone(),
        trust_override,
        // c490: product TUI Ask is `run_trust_gate_if_needed` (ChoicePrompt), never stdio.
        interactive: false,
        caller: if want_tui { "tui" } else { "cli" },
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
    #[cfg(feature = "tui")]
    let refuse_tui_untrusted = want_tui
        && bootstrapped
            .warnings
            .iter()
            .any(|w| matches!(w, BootstrapWarning::ProjectNotTrusted { .. }));
    let runtime = bootstrapped.into_runtime();
    let session_id = runtime.session_id;
    let project_trusted = !runtime
        .warnings
        .iter()
        .any(|w| matches!(w, BootstrapWarning::ProjectNotTrusted { .. }));
    let mut driver = runtime.driver;
    let cwd = std::env::current_dir().unwrap_or_default();
    let agent_dir = crate::infra::resource::DefaultResourceLoader::default_agent_dir();
    let mcp_servers = runtime.mcp_servers.unwrap_or_default();
    driver.enable_reload_state(cwd, agent_dir, project_trusted, mcp_servers);
    if let Err(e) = driver.bootstrap_mcp().await {
        eprintln!("Warning: MCP bootstrap failed: {e}");
    } else if let Some(summary) = driver.mcp_status_summary().await {
        // Surface connect issues (empty connected + diags) without breaking print-mode stdout.
        if summary.contains("diagnostics:") {
            eprintln!("Warning: {summary}");
        } else {
            log::info!(target: "xylitol::mcp", "{summary}");
        }
    }

    // ── dispatch by mode ───────────────────────────────────────
    #[cfg(feature = "tui")]
    if want_tui {
        // Align with pi / 图4: untrusted project → message already printed, no TUI.
        if refuse_tui_untrusted {
            return Ok(());
        }
        if let Err(e) = crate::app::tui::preflight(&driver) {
            eprintln!("Error: {e}");
            return Err(e.into());
        }
        return crate::app::tui::run(&mut driver)
            .await
            .map_err(|e| e.into());
    }

    let prompt = match resolve_print_prompt(
        one_shot.as_deref(),
        args.print || one_shot.is_none(),
        stdin_is_tty,
        || {
            use std::io::Read;
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            Ok(buf)
        },
    ) {
        Ok(p) => p,
        Err(msg) => {
            eprintln!("Error: {msg}");
            return Err(msg.into());
        }
    };

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_tty_selects_tui() {
        assert_eq!(
            select_surface_mode(false, false, false, true),
            SurfaceMode::Tui
        );
    }

    #[test]
    fn force_tui_wins_over_prompt() {
        assert_eq!(
            select_surface_mode(true, false, true, true),
            SurfaceMode::Tui
        );
    }

    #[test]
    fn prompt_selects_print() {
        assert_eq!(
            select_surface_mode(false, false, true, true),
            SurfaceMode::Print
        );
    }

    #[test]
    fn print_flag_selects_print() {
        assert_eq!(
            select_surface_mode(false, true, false, true),
            SurfaceMode::Print
        );
    }

    #[test]
    fn resolve_print_prompt_rejects_hello_fallback() {
        let err = resolve_print_prompt(None, false, true, || Ok(String::new())).unwrap_err();
        assert!(err.contains("--prompt") || err.contains("PROMPT"), "{err}");
        assert!(!err.contains("Hello!"), "{err}");
    }

    #[test]
    fn resolve_print_prompt_uses_flag() {
        let p = resolve_print_prompt(Some("hi"), false, true, || Ok(String::new())).unwrap();
        assert_eq!(p, "hi");
    }

    #[test]
    fn resolve_print_prompt_reads_pipe() {
        let p = resolve_print_prompt(None, true, false, || Ok("piped\n".into())).unwrap();
        assert_eq!(p, "piped");
    }
}
