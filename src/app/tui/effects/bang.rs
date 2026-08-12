//! Interactive bang loop shared by production and harness (c715 / ath9).

use std::time::Duration;

use futures::Stream;
use futures::StreamExt;
use xylitol_tui::Terminal;

use crate::app::core::driver::{EventStream, XyDriver, XyDriverError};

use super::super::commands::PendingBash;
use super::super::host::{HostEvent, HostSession};

/// Interactive bang loop shared by production `run_host_loop` and harness (c715 / ath9).
///
/// `input` yields terminal-side [`HostEvent`]s (Input / Paste / Resize). Tick is owned
/// here (16ms). Esc → `XyDriver::abort` + [`HostSession::note_bash_cancelled`] (not agent
/// Aborted). Empty `input` is valid for fire-and-forget bang that completes without Esc.
pub async fn run_interactive_bang<T, S>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
    bash: PendingBash,
    agent_stream: &mut Option<EventStream>,
    input: S,
) -> Result<(), XyDriverError>
where
    T: Terminal,
    S: Stream<Item = Result<HostEvent, XyDriverError>>,
{
    tokio::pin!(input);
    log::info!(target: "xylitol::tui", "XyDriver::execute_bash (interactive bang) command_len={} exclude={}", bash.command.len(), bash.exclude_from_context);
    session.begin_bash_exec(&bash.command, bash.exclude_from_context);
    let _ = session.render_now();
    let (chunk_tx, mut chunk_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(64);
    let (bash_result, aborted_during_bash) = {
        let bash_fut =
            driver.execute_bash(&bash.command, bash.exclude_from_context, Some(chunk_tx));
        tokio::pin!(bash_fut);
        let mut ticker = tokio::time::interval(Duration::from_millis(16));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut aborted_during_bash = false;
        let bash_result = loop {
            tokio::select! {
                biased;
                result = &mut bash_fut => break result,
                chunk = chunk_rx.recv() => {
                    if let Some(bytes) = chunk {
                        session.append_bash_chunk(&bytes);
                    }
                }
                _ = ticker.tick() => {
                    session.step(HostEvent::Tick)?;
                }
                maybe = input.next() => {
                    match maybe {
                        Some(Ok(ev)) => {
                            session.step(ev)?;
                            if session.take_abort() {
                                if aborted_during_bash {
                                    continue;
                                }
                                log::info!(
                                    target: "xylitol::tui",
                                    "XyDriver::abort during bang"
                                );
                                driver.abort();
                                session.note_bash_cancelled();
                                aborted_during_bash = true;
                                let _ = session.render_now();
                            }
                        }
                        Some(Err(e)) => {
                            e.log_failure("tui.bang.input");
                            session.tui.finish();
                            return Err(e);
                        }
                        None => {
                            session.request_quit();
                            let err = XyDriverError::message("input closed during bang");
                            err.log_failure("tui.bang.input_closed");
                            break Err(err);
                        }
                    }
                }
                maybe_agent = async {
                    match agent_stream.as_mut() {
                        Some(stream) => stream.next().await,
                        None => std::future::pending().await,
                    }
                } => {
                    match maybe_agent {
                        Some(xy) => {
                            session.step(HostEvent::Xy(Box::new(xy)))?;
                        }
                        None => {
                            log::debug!(
                                target: "xylitol::tui",
                                "agent EventStream ended during bang"
                            );
                            *agent_stream = None;
                            session.on_run_stream_closed();
                            let _ = session.tui.try_render();
                        }
                    }
                }
            }
        };
        (bash_result, aborted_during_bash)
    };
    match bash_result {
        Ok(r) => {
            if !aborted_during_bash {
                session.push_bash_result(&bash.command, &r);
            }
        }
        Err(e) => {
            e.log_failure("tui.execute_bash");
            session.push_scroll_notice(format!("bash failed: {e}"));
        }
    }
    session.end_bash_exec();
    if aborted_during_bash {
        let _ = driver.clear_queue(true, false);
        let stats = driver.queue_stats();
        session.set_queue_badge(stats.steer_count, stats.follow_up_count);
    }
    let _ = session.render_now();
    Ok(())
}
