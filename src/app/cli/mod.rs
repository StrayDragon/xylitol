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

/// Map `tui.activity_fold` config DTO → runtime knobs at the config-owning CLI
/// boundary (the TUI widget layer does not import infra config types, c2277).
#[cfg(feature = "tui")]
impl From<crate::infra::config::types::TuiActivityFoldConfig>
    for crate::app::tui::activity_fold::ActivityFoldSettings
{
    fn from(c: crate::infra::config::types::TuiActivityFoldConfig) -> Self {
        use crate::app::tui::activity_fold::SegmentLevel;
        use crate::infra::config::types::ActivityFoldStreamCollapse;
        let collapse_floor = match c.stream_collapse {
            ActivityFoldStreamCollapse::Envelope => SegmentLevel::L3,
            ActivityFoldStreamCollapse::Clusters => SegmentLevel::L2,
        };
        Self {
            enabled: c.enabled,
            keep_recent_turns: c.keep_recent_turns,
            auto_on_rebuild: c.auto_on_rebuild,
            auto_on_turn_end: c.auto_on_turn_end,
            collapse_floor,
        }
    }
}

use clap::{Args, Parser, Subcommand};

use crate::app::cli::resources::ResourcesAction;
use crate::app::cli::tokenizer::TokenizerAction;
use crate::app::core::bootstrap::{
    BootstrapError, BootstrapInput, BootstrapWarning, bootstrap, resolve_assembly,
};
use crate::app::core::driver::XyDriver;
#[cfg(feature = "server")]
use crate::app::server::subcommand::ServeAction;
use crate::infra::timing;

/// Surface flags for `xylitol tui` / `xylitol tui run` (c1565).
#[derive(Args, Debug, Default, Clone, PartialEq, Eq)]
pub struct TuiSurfaceArgs {
    #[arg(long)]
    pub session: Option<String>,
    #[arg(long)]
    pub model: Option<String>,
    #[arg(long)]
    pub config: Option<String>,
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
    /// Host origin to attach (`http://host:port`). Wins over `--port`.
    #[arg(long)]
    pub attach: Option<String>,
    /// Attach `http://127.0.0.1:<port>` when `--attach` is omitted.
    #[arg(long)]
    pub port: Option<u16>,
}

/// Optional leaf under `xylitol tui` (default = run when omitted).
/// Flags may sit on the parent (`tui --session …`) or on `run` (`tui run --session …`).
#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum TuiAction {
    /// Open the interactive TUI (same as bare `xylitol tui`).
    Run {
        #[command(flatten)]
        surface: TuiSurfaceArgs,
    },
}

/// Surface flags for `xylitol print` (c1565 / c1620).
#[derive(Args, Debug, Default, Clone, PartialEq, Eq)]
pub struct PrintSurfaceArgs {
    #[arg(long)]
    pub session: Option<String>,
    #[arg(long)]
    pub model: Option<String>,
    #[arg(long)]
    pub config: Option<String>,
    #[arg(long)]
    pub no_color: bool,
    /// Trust the project directory and load its `.xylitol/` resources.
    #[arg(long)]
    pub trust: bool,
    /// Do not trust the project directory; skip its `.xylitol/` resources.
    #[arg(long)]
    pub no_trust: bool,
}

/// Top-level subcommand. When absent: TTY → TUI; non-TTY → print (stdin).
/// Surface verbs (`tui` / `print`) vs ops (`resources` / …). No flat surface aliases.
#[derive(Subcommand, Debug)]
pub enum CliCommand {
    /// Interactive TUI surface (default on a TTY).
    Tui {
        #[command(flatten)]
        surface: TuiSurfaceArgs,
        #[command(subcommand)]
        action: Option<TuiAction>,
    },
    /// One-shot print surface (requires a non-empty prompt).
    Print {
        #[command(flatten)]
        surface: PrintSurfaceArgs,
        /// Positional one-shot prompt.
        #[arg(value_name = "PROMPT")]
        prompt: Option<String>,
        /// Prompt via flag (`--prompt` / `-p`) under `print` only.
        #[arg(short = 'p', long = "prompt", value_name = "TEXT")]
        prompt_flag: Option<String>,
    },
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
    /// Host listener (default 127.0.0.1:18790). Closing the TUI does not stop it.
    #[cfg(feature = "server")]
    Serve {
        /// Bind address.
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        /// Bind port (`0` lets the OS assign and prints the actual port).
        #[arg(long, default_value_t = 18790)]
        port: u16,
        #[command(subcommand)]
        action: Option<ServeAction>,
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
}

/// Resolved surface bootstrap knobs (from `tui` / `print` flatten args).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SurfaceBootstrap {
    pub session: Option<String>,
    pub model: Option<String>,
    pub config: Option<String>,
    pub list_models: bool,
    pub no_color: bool,
    pub trust: bool,
    pub no_trust: bool,
    pub attach: Option<String>,
    pub port: Option<u16>,
}

fn merge_tui_surface(parent: &TuiSurfaceArgs, action: Option<&TuiAction>) -> TuiSurfaceArgs {
    let Some(TuiAction::Run { surface: run }) = action else {
        return parent.clone();
    };
    // Prefer whichever side set a value (`tui --session x run` vs `tui run --session x`).
    TuiSurfaceArgs {
        session: run.session.clone().or_else(|| parent.session.clone()),
        model: run.model.clone().or_else(|| parent.model.clone()),
        config: run.config.clone().or_else(|| parent.config.clone()),
        list_models: run.list_models || parent.list_models,
        no_color: run.no_color || parent.no_color,
        trust: run.trust || parent.trust,
        no_trust: run.no_trust || parent.no_trust,
        attach: run.attach.clone().or_else(|| parent.attach.clone()),
        port: run.port.or(parent.port),
    }
}

/// Resolve surface-owned flags from a parsed CLI command (c1565).
pub fn surface_from_command(command: Option<&CliCommand>) -> SurfaceBootstrap {
    match command {
        Some(CliCommand::Tui { surface, action }) => {
            let s = merge_tui_surface(surface, action.as_ref());
            SurfaceBootstrap {
                session: s.session,
                model: s.model,
                config: s.config,
                list_models: s.list_models,
                no_color: s.no_color,
                trust: s.trust,
                no_trust: s.no_trust,
                attach: s.attach,
                port: s.port,
            }
        }
        Some(CliCommand::Print { surface, .. }) => SurfaceBootstrap {
            session: surface.session.clone(),
            model: surface.model.clone(),
            config: surface.config.clone(),
            list_models: false,
            no_color: surface.no_color,
            trust: surface.trust,
            no_trust: surface.no_trust,
            attach: None,
            port: None,
        },
        _ => SurfaceBootstrap::default(),
    }
}

fn merge_prompt_parts<'a>(flag: Option<&'a str>, positional: Option<&'a str>) -> Option<&'a str> {
    flag.or(positional).map(str::trim).filter(|s| !s.is_empty())
}

/// stderr resume line when the session was persisted (c1565).
pub fn resume_hint_line(session_id: Option<&str>, listed: bool) -> Option<String> {
    let sid = session_id?;
    listed.then(|| format!("Resume by $ xylitol tui --session {sid}"))
}

/// Resolve force-tui / explicit-print / one-shot prompt from surface verbs only.
///
/// Ops subcommands are handled before this; callers pass only `None` / `Tui` / `Print`.
/// There are no flat `--tui` / `--print` / top-level `-p` aliases.
pub fn resolve_surface_intent(command: Option<&CliCommand>) -> (bool, bool, Option<String>) {
    match command {
        Some(CliCommand::Tui { .. }) => (true, false, None),
        Some(CliCommand::Print {
            prompt,
            prompt_flag,
            ..
        }) => {
            let merged =
                merge_prompt_parts(prompt_flag.as_deref(), prompt.as_deref()).map(str::to_string);
            (false, true, merged)
        }
        None | Some(_) => (false, false, None),
    }
}

/// Which interactive surface to open (after `--list-models` is ruled out).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceMode {
    Tui,
    Print,
}

/// Pure dispatch: TUI default on a TTY; print via `print` verb or non-TTY bare launch.
pub fn select_surface_mode(
    force_tui: bool,
    explicit_print: bool,
    has_one_shot_prompt: bool,
    stdin_is_tty: bool,
) -> SurfaceMode {
    if force_tui {
        return SurfaceMode::Tui;
    }
    if has_one_shot_prompt || explicit_print {
        return SurfaceMode::Print;
    }
    if stdin_is_tty {
        return SurfaceMode::Tui;
    }
    // Non-TTY bare launch: treat as print so the caller can read stdin or error (no Hello!).
    SurfaceMode::Print
}

/// Resolve the print-mode prompt. Never returns a placeholder like `Hello!`.
#[derive(Debug, thiserror::Error)]
pub enum PrintPromptError {
    #[error("failed to read stdin: {0}")]
    Stdin(#[from] std::io::Error),
    #[error(
        "print mode requires a prompt: `xylitol print <PROMPT>`, `print --prompt TEXT`, \
         or pipe stdin (bare TTY launch opens the TUI; use `tui` to force it)"
    )]
    MissingPrompt,
}

pub fn resolve_print_prompt(
    one_shot: Option<&str>,
    allow_stdin_pipe: bool,
    stdin_is_tty: bool,
    mut read_stdin: impl FnMut() -> std::io::Result<String>,
) -> Result<String, PrintPromptError> {
    if let Some(p) = one_shot.map(str::trim).filter(|s| !s.is_empty()) {
        return Ok(p.to_string());
    }
    if allow_stdin_pipe && !stdin_is_tty {
        let buf = read_stdin()?;
        let trimmed = buf.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }
    Err(PrintPromptError::MissingPrompt)
}

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = CliArgs::parse();
    let surface = surface_from_command(args.command.as_ref());

    // Secrets first so `[otel]` / templates can see LANGFUSE_* from secret.env.
    let paths = crate::infra::config::paths::ConfigPaths::discover();
    let _ = crate::infra::config::secret_env::load_secret_env_files(&paths);

    // Best-effort early config for OTLP assembly (bootstrap reloads later).
    let early_otel = crate::infra::config::loader::load_app_config(
        surface.config.as_ref().map(std::path::Path::new),
    )
    .map(|c| c.otel)
    .unwrap_or_default();

    // ── Install file-only observability (fastrace + log) + optional OTLP ─
    // Done before any mode dispatch so every surface (print / TUI / RPC /
    // subcommands) is covered. Debug builds default on for local file sinks;
    // release needs RUST_LOG / XYLITOL_DEBUG / XYLITOL_PROVIDER_TRACE.
    // Remote OTLP: `[otel]` + feature `otel`, default none. Never stdout/stderr.
    logging::init_logging(
        &crate::infra::resource::DefaultResourceLoader::default_agent_dir(),
        &early_otel,
    );
    struct FlushOnDrop;
    impl Drop for FlushOnDrop {
        fn drop(&mut self) {
            logging::flush_observability();
        }
    }
    let _flush = FlushOnDrop;

    // ── Ops subcommands: early exit, no session/MCP bootstrap ───────
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
        Some(CliCommand::Serve { host, port, action }) => {
            return crate::app::server::subcommand::run(host, port, action).await;
        }
        Some(CliCommand::Tui { .. } | CliCommand::Print { .. }) | None => {}
    }

    timing::reset_timings();

    let trust_override = match (surface.trust, surface.no_trust) {
        (true, _) => Some(true),
        (_, true) => Some(false),
        _ => None,
    };

    use std::io::IsTerminal;
    let stdin_is_tty = std::io::stdin().is_terminal();
    let (force_tui, explicit_print, one_shot) = resolve_surface_intent(args.command.as_ref());

    #[cfg(feature = "tui")]
    let want_tui = !surface.list_models
        && select_surface_mode(force_tui, explicit_print, one_shot.is_some(), stdin_is_tty)
            == SurfaceMode::Tui;
    #[cfg(not(feature = "tui"))]
    let want_tui = false;

    #[cfg(all(feature = "tui", feature = "server"))]
    if want_tui {
        let url =
            crate::app::core::attach::resolve_attach_url(surface.attach.as_deref(), surface.port);
        if let Err(msg) = crate::app::core::attach::attach_preflight(&url).await {
            eprintln!("{msg}");
            return Err("host not listening".into());
        }
    }

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

    #[cfg(all(feature = "tui", feature = "server"))]
    if want_tui {
        return run_product_tui_attached(&surface).await;
    }

    let bootstrap_input = BootstrapInput {
        config_path: surface.config.as_ref().map(std::path::PathBuf::from),
        session: surface.session.clone(),
        model: surface.model.clone(),
        trust_override,
        // c490: product TUI Ask is `run_trust_gate_if_needed` (ChoicePrompt), never stdio.
        interactive: false,
        caller: "cli",
    };

    // `--list-models` needs the resolved registry before any agent build; it
    // resolves assembly (cheap — no agent construction) and returns early.
    if surface.list_models {
        let assembly = match resolve_assembly(&bootstrap_input) {
            Ok(a) => a,
            Err(BootstrapError::NoModelsAvailable) => {
                eprintln!(
                    "Error: {}",
                    provider_guidance::format_no_models_available_message()
                );
                return Err("no models available".into());
            }
            Err(BootstrapError::ConfigLoadFailed(e)) => {
                eprintln!("Error: config load failed ({e})");
                return Err(e.into());
            }
            Err(BootstrapError::ConfigLoadedZeroModels) => {
                eprintln!("Error: {}", BootstrapError::ConfigLoadedZeroModels);
                return Err("config loaded zero models".into());
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
        Err(BootstrapError::ConfigLoadFailed(e)) => {
            eprintln!("Error: config load failed ({e})");
            return Err(e.into());
        }
        Err(BootstrapError::ConfigLoadedZeroModels) => {
            eprintln!("Error: {}", BootstrapError::ConfigLoadedZeroModels);
            return Err("config loaded zero models".into());
        }
        Err(e) => return Err(e.into()),
    };
    render_warnings(&bootstrapped.warnings);
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
    #[cfg(all(feature = "tui", not(feature = "server")))]
    if want_tui {
        eprintln!("Error: product TUI requires the server feature to attach Host");
        return Err("tui requires server feature".into());
    }

    // print / non-TUI: settle MCP before the oneshot prompt (c1200).
    driver.begin_mcp_bootstrap().await;
    driver.wait_mcp_bootstrap().await;
    if let Some(summary) = driver.mcp_status_summary().await {
        if summary.contains("diagnostics:") {
            eprintln!("Warning: {summary}");
        } else {
            log::info!(target: "xylitol::mcp", "{summary}");
        }
    }

    let prompt = match resolve_print_prompt(
        one_shot.as_deref(),
        explicit_print || one_shot.is_none(),
        stdin_is_tty,
        || {
            use std::io::Read;
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            Ok(buf)
        },
    ) {
        Ok(p) => p,
        Err(err) => {
            eprintln!("Error: {err}");
            return Err(err.into());
        }
    };

    crate::app::cli::print::run_print(&mut driver, &prompt, &session_id).await?;
    maybe_print_resume_hint(&mut driver).await;

    timing::print_timings();
    Ok(())
}

#[cfg(all(feature = "tui", feature = "server"))]
async fn run_product_tui_attached(
    surface: &SurfaceBootstrap,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::IsTerminal;
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        eprintln!("Error: TUI requires a TTY");
        return Err("tui requires tty".into());
    }
    let attach_url =
        crate::app::core::attach::resolve_attach_url(surface.attach.as_deref(), surface.port);
    let mut driver = crate::app::core::driver::XyRemoteDriver::new(
        attach_url,
        surface.session.clone().unwrap_or_default(),
    );
    if let Some(model_id) = surface.model.as_deref() {
        crate::app::core::dispatch::dispatch(
            &mut driver,
            crate::protocol::Command::SetModel {
                provider: String::new(),
                model_id: model_id.to_string(),
            },
        )
        .await
        .map_err(|e| format!("--model {model_id}: {e}"))?;
    }
    let ask_gateway = std::sync::Arc::new(crate::app::tui::AskHostGateway::new());
    ask_gateway.set_host_client(std::sync::Arc::new(driver.host_client().clone()));
    let gw = ask_gateway.clone();
    driver.set_reverse_rpc_notify(std::sync::Arc::new(move |rpc_id, method, payload| {
        gw.push_from_server(rpc_id, &method, payload);
    }));
    let (seed_n, activity_fold) = match crate::infra::config::loader::load_app_config(
        surface.config.as_ref().map(std::path::Path::new),
    ) {
        Ok(c) => (
            c.tui.editor_history_seed_sessions,
            c.tui.activity_fold.into(),
        ),
        Err(_) => (
            1,
            crate::app::tui::activity_fold::ActivityFoldSettings::default(),
        ),
    };
    let tui_result = crate::app::tui::run(
        &mut driver,
        crate::app::tui::TuiRunOptions {
            editor_history_seed_sessions: seed_n,
            activity_fold,
            restored_session: surface.session.is_some(),
            ask_gateway: Some(ask_gateway),
            interaction_mode: crate::app::tui::lab_interaction_mode_from_env(),
        },
    )
    .await;
    maybe_print_resume_hint(&mut driver).await;
    tui_result.map_err(|e| e.into())
}

/// stderr resume line when the session was persisted (c1565).
async fn maybe_print_resume_hint(driver: &mut dyn crate::app::core::driver::XyDriver) {
    let sid = driver.session_id();
    let listed = match crate::app::core::dispatch::dispatch(
        driver,
        crate::protocol::Command::ListSessions {},
    )
    .await
    {
        Ok(crate::app::core::dispatch::DispatchOutcome::Sessions(list)) => sid
            .as_deref()
            .is_some_and(|id| list.iter().any(|e| e.id == id)),
        _ => false,
    };
    if let Some(line) = resume_hint_line(sid.as_deref(), listed) {
        eprintln!("{line}");
    }
}

/// Render bootstrap diagnostics as CLI warnings (print-mode byte-compat).
///
/// `provider_guidance` enriches the bare warning text (login help etc.) — this
/// is the single place where structured [`BootstrapWarning`]s become user-facing
/// CLI text, keeping `app::core::bootstrap` free of presentation deps.
fn render_warnings(warnings: &[BootstrapWarning]) {
    for w in warnings {
        match w {
            BootstrapWarning::ModelEntrySkipped(e) => {
                eprintln!("Warning: skipped model entry ({e})");
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
    use clap::Parser;

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
    fn explicit_print_selects_print() {
        assert_eq!(
            select_surface_mode(false, true, false, true),
            SurfaceMode::Print
        );
    }

    #[test]
    fn resolve_print_prompt_rejects_hello_fallback() {
        let err = resolve_print_prompt(None, false, true, || Ok(String::new())).unwrap_err();
        assert!(
            err.to_string().contains("print") || err.to_string().contains("PROMPT"),
            "{err}"
        );
        assert!(!err.to_string().contains("Hello!"), "{err}");
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

    #[test]
    fn parses_cli_command_tui() {
        let args = CliArgs::try_parse_from(["xylitol", "tui"]).unwrap();
        assert!(matches!(
            args.command,
            Some(CliCommand::Tui { action: None, .. })
        ));
        let (force_tui, explicit_print, one_shot) = resolve_surface_intent(args.command.as_ref());
        assert!(force_tui);
        assert!(!explicit_print);
        assert!(one_shot.is_none());
        assert_eq!(
            select_surface_mode(force_tui, explicit_print, one_shot.is_some(), true),
            SurfaceMode::Tui
        );
    }

    #[test]
    fn parses_cli_command_tui_run() {
        let args = CliArgs::try_parse_from(["xylitol", "tui", "run"]).unwrap();
        assert!(matches!(
            args.command,
            Some(CliCommand::Tui {
                action: Some(TuiAction::Run { .. }),
                ..
            })
        ));
    }

    #[test]
    fn tui_surface_flags_on_parent_or_run() {
        let a = CliArgs::try_parse_from(["xylitol", "tui", "--session", "abc"]).unwrap();
        let s = surface_from_command(a.command.as_ref());
        assert_eq!(s.session.as_deref(), Some("abc"));

        let b = CliArgs::try_parse_from(["xylitol", "tui", "run", "--session", "def"]).unwrap();
        let s = surface_from_command(b.command.as_ref());
        assert_eq!(s.session.as_deref(), Some("def"));

        let c = CliArgs::try_parse_from([
            "xylitol",
            "tui",
            "--model",
            "m1",
            "run",
            "--trust",
            "--list-models",
        ])
        .unwrap();
        let s = surface_from_command(c.command.as_ref());
        assert_eq!(s.model.as_deref(), Some("m1"));
        assert!(s.trust);
        assert!(s.list_models);
    }

    #[test]
    fn print_surface_flags_and_rejects_toplevel_surface_flags() {
        let a = CliArgs::try_parse_from([
            "xylitol",
            "print",
            "--session",
            "s1",
            "--model",
            "m",
            "--no-color",
            "hi",
        ])
        .unwrap();
        let s = surface_from_command(a.command.as_ref());
        assert_eq!(s.session.as_deref(), Some("s1"));
        assert_eq!(s.model.as_deref(), Some("m"));
        assert!(s.no_color);
        assert!(!s.list_models);
        assert!(!s.trust);

        let trusted = CliArgs::try_parse_from([
            "xylitol",
            "print",
            "--session",
            "s1",
            "--trust",
            "--model",
            "m",
            "hi",
        ])
        .unwrap();
        let s = surface_from_command(trusted.command.as_ref());
        assert!(s.trust);
        assert!(!s.no_trust);

        let denied =
            CliArgs::try_parse_from(["xylitol", "print", "--session", "s1", "--no-trust", "hi"])
                .unwrap();
        let s = surface_from_command(denied.command.as_ref());
        assert!(s.no_trust);
        assert!(!s.trust);

        for bad in [
            &["xylitol", "--session", "x"][..],
            &["xylitol", "--model", "m"][..],
            &["xylitol", "--list-models"][..],
            &["xylitol", "--trust"][..],
            &["xylitol", "--no-trust"][..],
            &["xylitol", "--config", "c.yaml"][..],
            &["xylitol", "--no-color"][..],
            &["xylitol", "--attach", "http://127.0.0.1:9"][..],
            &["xylitol", "--port", "9"][..],
        ] {
            assert!(
                CliArgs::try_parse_from(bad).is_err(),
                "top-level {:?} must be rejected",
                bad
            );
        }
    }

    #[test]
    fn parses_cli_command_print_with_prompt() {
        let args = CliArgs::try_parse_from(["xylitol", "print", "hello"]).unwrap();
        let (force_tui, explicit_print, one_shot) = resolve_surface_intent(args.command.as_ref());
        assert!(!force_tui);
        assert!(explicit_print);
        assert_eq!(one_shot.as_deref(), Some("hello"));
        assert_eq!(
            select_surface_mode(force_tui, explicit_print, one_shot.is_some(), true),
            SurfaceMode::Print
        );
    }

    #[test]
    fn print_verb_without_prompt_selects_print() {
        let args = CliArgs::try_parse_from(["xylitol", "print"]).unwrap();
        let (force_tui, explicit_print, one_shot) = resolve_surface_intent(args.command.as_ref());
        assert!(!force_tui);
        assert!(explicit_print);
        assert!(one_shot.is_none());
        let err =
            resolve_print_prompt(None, explicit_print, true, || Ok(String::new())).unwrap_err();
        assert!(!err.to_string().contains("Hello!"), "{err}");
    }

    #[test]
    fn rejects_flat_surface_aliases() {
        assert!(
            CliArgs::try_parse_from(["xylitol", "--tui"]).is_err(),
            "--tui must not exist"
        );
        assert!(
            CliArgs::try_parse_from(["xylitol", "--print"]).is_err(),
            "--print must not exist"
        );
        assert!(
            CliArgs::try_parse_from(["xylitol", "-p", "x"]).is_err(),
            "top-level -p must not exist"
        );
        assert!(
            CliArgs::try_parse_from(["xylitol", "hello"]).is_err(),
            "top-level positional prompt must not exist"
        );
        // `-p` lives only under `print`
        let args = CliArgs::try_parse_from(["xylitol", "print", "-p", "x"]).unwrap();
        let (_, explicit_print, one_shot) = resolve_surface_intent(args.command.as_ref());
        assert!(explicit_print);
        assert_eq!(one_shot.as_deref(), Some("x"));
    }

    #[test]
    fn resume_hint_line_only_when_listed() {
        assert_eq!(
            resume_hint_line(Some("abc"), true).as_deref(),
            Some("Resume by $ xylitol tui --session abc")
        );
        assert!(resume_hint_line(Some("abc"), false).is_none());
        assert!(resume_hint_line(None, true).is_none());
    }

    #[test]
    fn help_lists_surface_and_ops_commands() {
        use clap::CommandFactory;
        let mut cmd = CliArgs::command();
        let help = cmd.render_long_help().to_string();
        for name in ["tui", "print", "resources", "tokenizer"] {
            assert!(
                help.contains(name),
                "expected `{name}` in top-level help:\n{help}"
            );
        }
        #[cfg(feature = "server")]
        {
            assert!(
                cmd.find_subcommand("serve").is_some(),
                "expected serve in top-level help:\n{help}"
            );
            assert!(
                cmd.find_subcommand("server").is_none(),
                "server must not remain as a product verb:\n{help}"
            );
        }
        assert!(
            !help.contains("--tui") && !help.contains("--print"),
            "flat surface flags must be gone:\n{help}"
        );
        let tui = cmd.find_subcommand_mut("tui").expect("tui");
        let tui_help = tui.render_long_help().to_string();
        assert!(tui_help.contains("run"), "{tui_help}");
        assert!(
            tui.find_subcommand("tokenizer").is_none()
                && tui.find_subcommand("resources").is_none(),
            "ops must stay top-level, not under tui:\n{tui_help}"
        );
        assert!(
            tui_help.contains("--session")
                && tui_help.contains("--trust")
                && tui_help.contains("--attach")
                && tui_help.contains("--port"),
            "tui must own surface flags:\n{tui_help}"
        );
    }

    #[cfg(feature = "server")]
    #[test]
    fn parses_cli_serve_and_rejects_aliases() {
        let listen = CliArgs::try_parse_from(["xylitol", "serve"]).unwrap();
        match listen.command {
            Some(CliCommand::Serve {
                host,
                port,
                action: None,
            }) => {
                assert_eq!(host, "127.0.0.1");
                assert_eq!(port, 18790);
            }
            other => panic!("expected bare serve, got {other:?}"),
        }

        let custom =
            CliArgs::try_parse_from(["xylitol", "serve", "--host", "0.0.0.0", "--port", "0"])
                .unwrap();
        match custom.command {
            Some(CliCommand::Serve {
                host,
                port,
                action: None,
            }) => {
                assert_eq!(host, "0.0.0.0");
                assert_eq!(port, 0);
            }
            other => panic!("expected serve flags, got {other:?}"),
        }

        let stop = CliArgs::try_parse_from(["xylitol", "serve", "stop"]).unwrap();
        assert!(matches!(
            stop.command,
            Some(CliCommand::Serve {
                action: Some(ServeAction::Stop),
                ..
            })
        ));
        let install = CliArgs::try_parse_from(["xylitol", "serve", "install"]).unwrap();
        assert!(matches!(
            install.command,
            Some(CliCommand::Serve {
                action: Some(ServeAction::Install),
                ..
            })
        ));

        assert!(
            CliArgs::try_parse_from(["xylitol", "server"]).is_err(),
            "server alias must not parse"
        );
        assert!(
            CliArgs::try_parse_from(["xylitol", "serve", "run"]).is_err(),
            "serve run must not parse"
        );
    }

    #[test]
    fn tui_attach_flags_and_print_rejects_them() {
        let a =
            CliArgs::try_parse_from(["xylitol", "tui", "--attach", "http://127.0.0.1:9"]).unwrap();
        let s = surface_from_command(a.command.as_ref());
        assert_eq!(s.attach.as_deref(), Some("http://127.0.0.1:9"));

        let b = CliArgs::try_parse_from(["xylitol", "tui", "--port", "9"]).unwrap();
        let s = surface_from_command(b.command.as_ref());
        assert_eq!(s.port, Some(9));
        assert_eq!(
            crate::app::core::attach::resolve_attach_url(s.attach.as_deref(), s.port),
            "http://127.0.0.1:9"
        );

        let c = CliArgs::try_parse_from([
            "xylitol",
            "tui",
            "--port",
            "9",
            "--attach",
            "http://127.0.0.1:77",
        ])
        .unwrap();
        let s = surface_from_command(c.command.as_ref());
        assert_eq!(
            crate::app::core::attach::resolve_attach_url(s.attach.as_deref(), s.port),
            "http://127.0.0.1:77"
        );

        let d = CliArgs::try_parse_from(["xylitol", "tui", "run", "--port", "11"]).unwrap();
        let s = surface_from_command(d.command.as_ref());
        assert_eq!(s.port, Some(11));

        assert!(
            CliArgs::try_parse_from(["xylitol", "print", "--attach", "http://127.0.0.1:9", "hi"])
                .is_err()
        );
        assert!(CliArgs::try_parse_from(["xylitol", "print", "--port", "9", "hi"]).is_err());
        assert!(CliArgs::try_parse_from(["xylitol", "--attach", "http://127.0.0.1:9"]).is_err());
        assert!(CliArgs::try_parse_from(["xylitol", "--port", "9"]).is_err());
    }
}
