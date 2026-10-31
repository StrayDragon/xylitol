use std::time::Duration;

use futures::Stream;
use futures::StreamExt;
use xylitol_tui::Terminal;

use crate::driver::{XyDriver, XyDriverError};

use super::super::commands::{RELOAD_CANCELLED_NOTICE, RELOAD_FAILED_NOTICE};
use super::super::host::{HostEvent, HostSession};
use super::super::keybindings::{ReloadOutcome, default_agent_dir};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ClientReloadPhase {
    BeforeHost,
    AfterHost,
}

pub struct ClientReloadStep<T: Terminal> {
    pub name: &'static str,
    pub phase: ClientReloadPhase,
    pub run: Box<dyn Fn(&mut HostSession<T>) -> Result<String, String>>,
}

pub fn default_client_reload_steps<T: Terminal>() -> Vec<ClientReloadStep<T>> {
    vec![
        ClientReloadStep {
            name: "keybindings",
            phase: ClientReloadPhase::BeforeHost,
            run: Box::new(|session| {
                Ok(format_keybindings_reload(
                    session.reload_keybindings(&default_agent_dir()),
                ))
            }),
        },
        ClientReloadStep {
            name: "themes",
            phase: ClientReloadPhase::AfterHost,
            run: Box::new(|session| {
                let Some(pref) = session.theme_preference().map(str::to_string) else {
                    return Ok("themes: unchanged (no preference; kept current)".into());
                };
                match session.reload_themes(&pref) {
                    Ok(()) => Ok(format!("themes: ok — kept `{pref}`")),
                    Err(e) => {
                        e.log_failure("tui.reload_themes");
                        Err(e.to_string())
                    }
                }
            }),
        },
    ]
}

fn format_keybindings_reload(outcome: ReloadOutcome) -> String {
    match outcome {
        ReloadOutcome::Applied { path } => format!("keybindings: ok — {}", path.display()),
        ReloadOutcome::NoFile { path } => {
            format!("keybindings: ok (no file) — {}", path.display())
        }
        ReloadOutcome::Failed { path, error } => {
            format!("keybindings: failed — {} ({error})", path.display())
        }
    }
}

fn reload_notice_title(cancelled: bool) -> &'static str {
    if cancelled {
        "Reload cancelled:"
    } else {
        "Reload:"
    }
}

fn run_client_reload_step<T: Terminal>(
    session: &mut HostSession<T>,
    step: &ClientReloadStep<T>,
) -> String {
    match (step.run)(session) {
        Ok(message) => message,
        Err(e) => format!("{}: failed — {e}", step.name),
    }
}

pub async fn run_interactive_reload<T, S>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
    input: S,
    steps: &[ClientReloadStep<T>],
) -> Result<(), XyDriverError>
where
    T: Terminal,
    S: Stream<Item = Result<HostEvent, XyDriverError>>,
{
    tokio::pin!(input);

    session.begin_reload();
    let _ = session.render_now();

    let Some(cancel) = session.reload_cancel_token() else {
        session.end_reload();
        return Ok(());
    };

    let mut lines = vec![String::new()];
    for step in steps
        .iter()
        .filter(|step| step.phase == ClientReloadPhase::BeforeHost)
    {
        lines.push(run_client_reload_step(session, step));
    }

    let runtime_result = {
        let reload_fut = driver.reload_runtime(&cancel);
        tokio::pin!(reload_fut);

        let mut ticker = tokio::time::interval(Duration::from_millis(16));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut step_err: Option<XyDriverError> = None;

        let report = loop {
            tokio::select! {
                biased;
                result = &mut reload_fut => break result,
                _ = ticker.tick() => {
                    if let Err(e) = session.step(HostEvent::Tick) {
                        step_err.get_or_insert(e);
                        cancel.cancel();
                    }
                    let _ = session.render_now();
                }
                maybe = input.next() => {
                    match maybe {
                        Some(Ok(ev)) => {
                            if let Err(e) = session.step(ev) {
                                step_err.get_or_insert(e);
                                cancel.cancel();
                            }
                            let _ = session.render_now();
                        }
                        Some(Err(e)) => {
                            e.log_failure("tui.reload.input");
                            step_err.get_or_insert(e);
                            cancel.cancel();
                        }
                        None => {
                            cancel.cancel();
                        }
                    }
                }
            }
        };
        (report, step_err)
    };
    let (runtime_result, step_err) = runtime_result;
    if let Some(e) = step_err {
        session.end_reload();
        return Err(e);
    }

    match runtime_result {
        Ok(report) => {
            lines[0] = reload_notice_title(report.cancelled).to_string();
            lines.extend(report.format_lines());
            if report.cancelled {
                session.push_toast_notice(RELOAD_CANCELLED_NOTICE);
            } else if report.any_step_failed() {
                session.push_toast_notice(RELOAD_FAILED_NOTICE);
            }
        }
        Err(e) => {
            e.log_failure("tui.reload_runtime");
            lines[0] = "Reload:".into();
            lines.push(format!("runtime: failed — {e}"));
            session.push_toast_notice(RELOAD_FAILED_NOTICE);
        }
    }

    for step in steps
        .iter()
        .filter(|step| step.phase == ClientReloadPhase::AfterHost)
    {
        lines.push(run_client_reload_step(session, step));
    }

    session.set_model_arg_catalog_from_models(&driver.available_models());
    session.set_dollar_skill_catalog(driver.dollar_skill_catalog());
    session.set_plugin_commands(
        driver
            .get_commands()
            .into_iter()
            .map(|c| (c.name, c.description))
            .collect(),
    );
    session.refresh_loaded_resources(driver).await;
    session.push_scroll_notice(lines.join("\n"));
    session.end_reload();
    let _ = session.render_now();
    Ok(())
}
