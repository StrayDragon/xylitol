mod args;
mod intent;
pub use xylitol_official_plugin_obs as logging;
mod print;
pub use args::*;
pub use intent::*;
pub use print::render_stream;
pub use print::{format_tool_end_line, format_tool_start_lines};
pub use provider_guidance::{
    format_no_api_key_found_message, format_no_models_available_message, get_provider_login_help,
};
mod provider_guidance;
#[cfg(feature = "tui")]
mod trust_gate;

use clap::Parser;

use xylitol_kernel::boot::{BootOptions, Profile, boot};
use xylitol_kernel::bootstrap::{
    BootstrapError, BootstrapInput, BootstrapWarning, resolve_assembly,
};
use xylitol_kernel::driver::XyDriver;
#[cfg(all(test, feature = "server"))]
use xylitol_kernel::occupancy::subcommand::ServeAction;
use xylitol_kernel::timing;

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = CliArgs::parse();
    let surface = surface_from_command(args.command.as_ref());

    let paths = xylitol_kernel::config::paths::ConfigPaths::discover();
    let _ = xylitol_kernel::config::secret_env::load_secret_env_files(&paths);

    let early_otel = xylitol_kernel::config::loader::load_app_config(
        surface.config.as_ref().map(std::path::Path::new),
    )
    .map(|c| c.otel)
    .unwrap_or_default();

    logging::init_logging(
        &xylitol_runtime::resource::DefaultResourceLoader::default_agent_dir(),
        &early_otel,
    );
    struct FlushOnDrop;
    impl Drop for FlushOnDrop {
        fn drop(&mut self) {
            logging::flush_observability();
        }
    }
    let _flush = FlushOnDrop;

    match args.command {
        #[cfg(feature = "server")]
        Some(CliCommand::Serve { host, port, action }) => {
            return xylitol_kernel::occupancy::subcommand::run(host, port, action).await;
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
            xylitol_kernel::attach::resolve_attach_url(surface.attach.as_deref(), surface.port);
        if let Err(msg) = xylitol_kernel::attach::attach_preflight(&url).await {
            eprintln!("{msg}");
            return Err("host not listening".into());
        }
    }

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
        return run_product_tui_attached(&surface, is_tui_designing(args.command.as_ref())).await;
    }

    let bootstrap_input = BootstrapInput {
        config_path: surface.config.as_ref().map(std::path::PathBuf::from),
        session: surface.session.clone(),
        model: surface.model.clone(),
        trust_override,
        interactive: false,
        caller: "cli",
        ephemeral_sessions: false,
        static_tools: Vec::new(),
    };

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

    let kernel = match boot(BootOptions {
        profile: Profile::Print,
        config_path: bootstrap_input.config_path,
        session: bootstrap_input.session,
        model: bootstrap_input.model,
        trust_override: bootstrap_input.trust_override,
        interactive: bootstrap_input.interactive,
        caller: bootstrap_input.caller,
        ephemeral_sessions: bootstrap_input.ephemeral_sessions,
    })
    .await
    {
        Ok(k) => k,
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
    render_warnings(&kernel.runtime.warnings);
    let runtime = kernel.runtime;
    let session_id = runtime.session_id;
    let project_trusted = !runtime
        .warnings
        .iter()
        .any(|w| matches!(w, BootstrapWarning::ProjectNotTrusted { .. }));
    let mut driver = runtime.driver;
    let cwd = std::env::current_dir().unwrap_or_default();
    let agent_dir = xylitol_runtime::resource::DefaultResourceLoader::default_agent_dir();
    driver.enable_reload_state(cwd, agent_dir, project_trusted);
    #[cfg(all(feature = "tui", not(feature = "server")))]
    if want_tui {
        eprintln!("Error: product TUI requires the server feature to attach Host");
        return Err("tui requires server feature".into());
    }

    driver.begin_session_prepare().await;
    driver.wait_session_prepare().await;
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

    crate::cli::print::run_print(&mut driver, &prompt, &session_id).await?;
    maybe_print_resume_hint(&mut driver).await;

    timing::print_timings();
    Ok(())
}

#[cfg(all(feature = "tui", feature = "server"))]
async fn run_product_tui_attached(
    surface: &SurfaceBootstrap,
    designing: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::IsTerminal;
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        eprintln!("Error: TUI requires a TTY");
        return Err("tui requires tty".into());
    }
    let attach_url =
        xylitol_kernel::attach::resolve_attach_url(surface.attach.as_deref(), surface.port);
    let mut driver = xylitol_app_tui::XyRemoteDriver::new(
        attach_url,
        surface.session.clone().unwrap_or_default(),
    );
    if let Some(model_id) = surface.model.as_deref() {
        xylitol_app_tui::dispatch::dispatch(
            &mut driver,
            xylitol_protocol::Command::SetModel {
                provider: String::new(),
                model_id: model_id.to_string(),
            },
        )
        .await
        .map_err(|e| format!("--model {model_id}: {e}"))?;
    }
    let ask_gateway = std::sync::Arc::new(xylitol_app_tui::AskHostGateway::new());
    ask_gateway.set_host_client(std::sync::Arc::new(driver.host_client().clone()));
    let gw = ask_gateway.clone();
    driver.set_reverse_rpc_notify(std::sync::Arc::new(move |rpc_id, method, payload| {
        gw.push_from_server(rpc_id, &method, payload);
    }));
    let (seed_n, activity_fold) = match xylitol_kernel::config::loader::load_app_config(
        surface.config.as_ref().map(std::path::Path::new),
    ) {
        Ok(c) => (
            c.tui.editor_history_seed_sessions,
            crate::cli::activity_fold_settings(c.tui.activity_fold),
        ),
        Err(_) => (1, xylitol_app_tui::ActivityFoldSettings::default()),
    };
    let tui_result = xylitol_app_tui::run(
        &mut driver,
        xylitol_app_tui::TuiRunOptions {
            editor_history_seed_sessions: seed_n,
            activity_fold,
            restored_session: surface.session.is_some(),
            ask_gateway: Some(ask_gateway),
            client_storage: Some(std::sync::Arc::new(
                xylitol_app_tui::storage::ClientStorage::from_env(),
            )),
            interaction_mode: xylitol_app_tui::lab_interaction_mode_from_env(),
            designing,
        },
    )
    .await;
    maybe_print_resume_hint_tui(&mut driver).await;
    tui_result.map_err(|e| e.into())
}

#[cfg(all(feature = "tui", feature = "server"))]
async fn maybe_print_resume_hint_tui(driver: &mut dyn xylitol_app_tui::XyDriver) {
    let sid = driver.session_id();
    let listed = match xylitol_app_tui::dispatch::dispatch(
        driver,
        xylitol_protocol::Command::ListSessions {},
    )
    .await
    {
        Ok(xylitol_app_tui::dispatch::DispatchOutcome::Sessions(list)) => sid
            .as_deref()
            .is_some_and(|id| list.iter().any(|e| e.id == id)),
        _ => false,
    };
    if let Some(line) = resume_hint_line(sid.as_deref(), listed) {
        eprintln!("{line}");
    }
}

async fn maybe_print_resume_hint<D>(driver: &mut D)
where
    D: xylitol_kernel::driver::XyDriver + xylitol_kernel::dispatch::SessionCommandExecutor,
{
    let sid = driver.session_id();
    let listed = match xylitol_kernel::dispatch::dispatch(
        driver,
        xylitol_protocol::Command::ListSessions {},
    )
    .await
    {
        Ok(xylitol_kernel::dispatch::DispatchOutcome::Sessions(list)) => sid
            .as_deref()
            .is_some_and(|id| list.iter().any(|e| e.id == id)),
        _ => false,
    };
    if let Some(line) = resume_hint_line(sid.as_deref(), listed) {
        eprintln!("{line}");
    }
}

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
    fn help_omits_toplevel_preview_and_designing() {
        use clap::CommandFactory;
        let mut cmd = CliArgs::command();
        assert!(cmd.find_subcommand("preview").is_none());
        assert!(cmd.find_subcommand("designing").is_none());
        let help = cmd.render_long_help().to_string();
        assert!(!help.contains("preview"), "{help}");
        let commands_block = help.split("Commands:").nth(1).unwrap_or(&help);
        assert!(!commands_block.contains("designing"), "{commands_block}");
    }

    #[test]
    fn tui_designing_flag_is_not_a_surface_flag() {
        assert!(CliArgs::try_parse_from(["xylitol", "tui", "--designing"]).is_err());
    }

    #[cfg(debug_assertions)]
    #[test]
    fn debug_tui_designing_parses_and_attaches() {
        use clap::CommandFactory;
        let args = CliArgs::try_parse_from(["xylitol", "tui", "designing"]).unwrap();
        assert!(is_tui_designing(args.command.as_ref()));
        assert!(tui_requires_host_listener(args.command.as_ref()));
        let trusted =
            CliArgs::try_parse_from(["xylitol", "tui", "--trust", "--port", "9", "designing"])
                .unwrap();
        let surface = surface_from_command(trusted.command.as_ref());
        assert!(surface.trust, "tui --trust designing must set trust");
        assert_eq!(surface.port, Some(9));
        let mut cmd = CliArgs::command();
        let tui = cmd.find_subcommand_mut("tui").expect("tui");
        assert!(tui.find_subcommand("designing").is_some());
        let help = tui.render_long_help().to_string();
        assert!(help.contains("designing"), "{help}");
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn release_tui_omits_designing() {
        use clap::CommandFactory;
        assert!(CliArgs::try_parse_from(["xylitol", "tui", "designing"]).is_err());
        let mut cmd = CliArgs::command();
        let tui = cmd.find_subcommand_mut("tui").expect("tui");
        assert!(tui.find_subcommand("designing").is_none());
        let help = tui.render_long_help().to_string();
        assert!(!help.contains("designing"), "{help}");
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
        for name in ["tui", "print"] {
            assert!(
                help.contains(name),
                "expected `{name}` in top-level help:\n{help}"
            );
        }
        assert!(
            cmd.find_subcommand("tokenizer").is_none(),
            "tokenizer must not remain a product verb:\n{help}"
        );
        assert!(
            cmd.find_subcommand("resources").is_none(),
            "resources must not remain a product verb:\n{help}"
        );
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
                && tui.find_subcommand("resources").is_none()
                && tui.find_subcommand("serve").is_none(),
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
        assert!(
            CliArgs::try_parse_from(["xylitol", "serve", "install"]).is_err(),
            "serve install must not parse"
        );

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
            xylitol_kernel::attach::resolve_attach_url(s.attach.as_deref(), s.port),
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
            xylitol_kernel::attach::resolve_attach_url(s.attach.as_deref(), s.port),
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
