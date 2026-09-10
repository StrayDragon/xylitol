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
    log::info!(target: "xylitol::tui", "Command::Bash (interactive bang) command_len={} exclude={}", bash.command.len(), bash.exclude_from_context);
    session.begin_bash_exec(&bash.command, bash.exclude_from_context);
    let _ = session.render_now();
    // c2760: inject the output-event sink and dispatch `Command::Bash`. The
    // sink channel is independent of the dispatch borrow, so abort stays a
    // simple post-loop action; the client-side cancel clone is inert (the
    // host's slot-registered token does the real kill).
    let (chunk_tx, mut chunk_rx) =
        tokio::sync::mpsc::channel::<crate::protocol::ports::BashChunk>(64);
    driver.set_bash_run_sink(Some(crate::protocol::ports::BashOutputSink {
        tx: chunk_tx,
        cancel: tokio_util::sync::CancellationToken::new(),
    }));
    let (bash_result, aborted_during_bash) = {
        let cmd = crate::protocol::Command::Bash {
            command: bash.command.clone(),
            exclude_from_context: bash.exclude_from_context,
        };
        let mut dispatch_fut = Box::pin(crate::app::core::dispatch::dispatch(driver, cmd));
        let mut ticker = tokio::time::interval(Duration::from_millis(16));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut abort_requested = false;
        let mut chunks_done = false;
        let result = loop {
            tokio::select! {
                biased;
                result = &mut dispatch_fut => break Some(result),
                chunk = chunk_rx.recv(), if !chunks_done => {
                    match chunk {
                        Some(chunk) => {
                            session.append_bash_chunk(chunk.data.as_bytes());
                        }
                        None => {
                            chunks_done = true;
                        }
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
                                // Abort after the select releases the dispatch
                                // borrow: cancel the run loop / kill the bash tree.
                                log::info!(target: "xylitol::tui", "Command::Abort during bang");
                                abort_requested = true;
                                break None;
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
                            break Some(Err(err));
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
        drop(dispatch_fut);
        driver.set_bash_run_sink(None);
        if abort_requested {
            driver.abort();
            session.note_bash_cancelled();
            let _ = session.render_now();
        }
        (result, abort_requested)
    };
    // Unwrap the dispatch outcome into the bash result shape consumed below.
    let bash_result = match bash_result {
        Some(Ok(crate::app::core::dispatch::DispatchOutcome::Bash(r))) => Ok(r),
        Some(Ok(other)) => Err(XyDriverError::message(format!(
            "Command::Bash unexpected outcome: {other:?}"
        ))),
        Some(Err(e)) => Err(e),
        None => Err(XyDriverError::message("bash aborted")),
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
        let _ = crate::app::core::dispatch::dispatch(
            driver,
            crate::protocol::Command::ClearQueue {
                clear_steer: true,
                clear_follow_up: false,
            },
        )
        .await;
        let stats = super::queue_stats(driver).await;
        session.set_queue_badge(stats.steer_count, stats.follow_up_count);
    }
    let _ = session.render_now();
    Ok(())
}
