use super::args::{CliCommand, SurfaceBootstrap, TuiAction, TuiSurfaceArgs};

const TUI_PARENT_ARGS: &str = "the parent `xylitol tui` command";
const TUI_RUN_ARGS: &str = "the `xylitol tui run` subcommand";

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "conflicting trust flags: --trust and --no-trust must not be combined (--trust from {trust_at}, --no-trust from {no_trust_at})"
)]
pub struct TrustFlagConflict {
    trust_at: &'static str,
    no_trust_at: &'static str,
}

impl TrustFlagConflict {
    fn same_command() -> Self {
        Self {
            trust_at: "the same command",
            no_trust_at: "the same command",
        }
    }
}

pub fn resolve_trust_override(
    trust: bool,
    no_trust: bool,
) -> Result<Option<bool>, TrustFlagConflict> {
    match (trust, no_trust) {
        (false, false) => Ok(None),
        (true, false) => Ok(Some(true)),
        (false, true) => Ok(Some(false)),
        (true, true) => Err(TrustFlagConflict::same_command()),
    }
}

fn check_merged_trust_flags(
    trust: bool,
    no_trust: bool,
    trust_at: &'static str,
    no_trust_at: &'static str,
) -> Result<(), TrustFlagConflict> {
    if trust && no_trust {
        return Err(TrustFlagConflict {
            trust_at,
            no_trust_at,
        });
    }
    Ok(())
}

fn check_tui_trust_pair(
    parent: &TuiSurfaceArgs,
    run: &TuiSurfaceArgs,
) -> Result<(), TrustFlagConflict> {
    check_merged_trust_flags(
        run.trust || parent.trust,
        run.no_trust || parent.no_trust,
        if run.trust {
            TUI_RUN_ARGS
        } else {
            TUI_PARENT_ARGS
        },
        if run.no_trust {
            TUI_RUN_ARGS
        } else {
            TUI_PARENT_ARGS
        },
    )
}

pub fn is_tui_designing(command: Option<&CliCommand>) -> bool {
    #[cfg(debug_assertions)]
    {
        matches!(
            command,
            Some(CliCommand::Tui {
                action: Some(TuiAction::Designing),
                ..
            })
        )
    }
    #[cfg(not(debug_assertions))]
    {
        let _ = command;
        false
    }
}

pub fn tui_requires_host_listener(command: Option<&CliCommand>) -> bool {
    matches!(command, Some(CliCommand::Tui { .. }) | None)
}

fn merge_tui_surface(
    parent: &TuiSurfaceArgs,
    action: Option<&TuiAction>,
) -> Result<TuiSurfaceArgs, TrustFlagConflict> {
    let Some(TuiAction::Run { surface: run }) = action else {
        return Ok(parent.clone());
    };
    check_tui_trust_pair(parent, run)?;
    Ok(TuiSurfaceArgs {
        session: run.session.clone().or_else(|| parent.session.clone()),
        model: run.model.clone().or_else(|| parent.model.clone()),
        agent_profile: run
            .agent_profile
            .clone()
            .or_else(|| parent.agent_profile.clone()),
        config: run.config.clone().or_else(|| parent.config.clone()),
        list_models: run.list_models || parent.list_models,
        no_color: run.no_color || parent.no_color,
        trust: run.trust || parent.trust,
        no_trust: run.no_trust || parent.no_trust,
        attach: run.attach.clone().or_else(|| parent.attach.clone()),
        port: run.port.or(parent.port),
        no_ensure_host: run.no_ensure_host || parent.no_ensure_host,
    })
}

pub fn surface_from_command(
    command: Option<&CliCommand>,
) -> Result<SurfaceBootstrap, TrustFlagConflict> {
    match command {
        Some(CliCommand::Tui { surface, action }) => {
            let s = merge_tui_surface(surface, action.as_ref())?;
            Ok(SurfaceBootstrap {
                session: s.session,
                model: s.model,
                agent_profile: s.agent_profile,
                config: s.config,
                list_models: s.list_models,
                no_color: s.no_color,
                trust: s.trust,
                no_trust: s.no_trust,
                attach: s.attach,
                port: s.port,
                no_ensure_host: s.no_ensure_host,
            })
        }
        Some(CliCommand::Print { surface, .. }) => {
            resolve_trust_override(surface.trust, surface.no_trust)?;
            Ok(SurfaceBootstrap {
                session: surface.session.clone(),
                model: surface.model.clone(),
                agent_profile: surface.agent_profile.clone(),
                config: surface.config.clone(),
                list_models: false,
                no_color: surface.no_color,
                trust: surface.trust,
                no_trust: surface.no_trust,
                attach: None,
                port: None,
                no_ensure_host: false,
            })
        }
        _ => Ok(SurfaceBootstrap::default()),
    }
}

fn merge_prompt_parts<'a>(flag: Option<&'a str>, positional: Option<&'a str>) -> Option<&'a str> {
    flag.or(positional).map(str::trim).filter(|s| !s.is_empty())
}

pub fn resume_hint_line(session_id: Option<&str>, listed: bool) -> Option<String> {
    let sid = session_id?;
    listed.then(|| format!("Resume by $ xylitol tui --session {sid}"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceIntent {
    pub force_tui: bool,
    pub explicit_print: bool,
    pub one_shot_prompt: Option<String>,
}

impl SurfaceIntent {
    pub fn from_command(command: Option<&CliCommand>) -> Self {
        match command {
            Some(CliCommand::Tui { .. }) => Self {
                force_tui: true,
                explicit_print: false,
                one_shot_prompt: None,
            },
            Some(CliCommand::Print {
                prompt,
                prompt_flag,
                ..
            }) => Self {
                force_tui: false,
                explicit_print: true,
                one_shot_prompt: merge_prompt_parts(prompt_flag.as_deref(), prompt.as_deref())
                    .map(str::to_string),
            },
            None | Some(_) => Self {
                force_tui: false,
                explicit_print: false,
                one_shot_prompt: None,
            },
        }
    }

    pub fn wants_tui(&self, list_models: bool, stdin_is_tty: bool) -> bool {
        !list_models && self.mode(stdin_is_tty) == SurfaceMode::Tui
    }

    pub fn mode(&self, stdin_is_tty: bool) -> SurfaceMode {
        if self.force_tui {
            return SurfaceMode::Tui;
        }
        if self.explicit_print || self.one_shot_prompt.is_some() {
            return SurfaceMode::Print;
        }
        if stdin_is_tty {
            SurfaceMode::Tui
        } else {
            SurfaceMode::Print
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceMode {
    Tui,
    Print,
}

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
