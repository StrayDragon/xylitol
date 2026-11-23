use futures::StreamExt;
use xylitol_tui::Terminal;

use crate::driver::{EventStream, XyDriver, XyDriverError};

pub use super::effects::{
    default_client_reload_steps, drain_pending, refresh_footer_tokens, run_interactive_bang,
    run_interactive_reload,
};
use super::host::{HostEvent, HostSession};

mod scripted;
pub use scripted::ScriptedDriver;

pub async fn pump_host_driver<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
    agent_stream: &mut Option<EventStream>,
) -> Result<(), XyDriverError> {
    drain_pending(session, driver, agent_stream).await?;

    if session.take_reload() {
        let input = futures::stream::pending::<Result<HostEvent, XyDriverError>>();
        let steps = default_client_reload_steps();
        run_interactive_reload(session, driver, input, &steps).await?;
    }

    if let Some(bash) = session.take_bash() {
        let input = futures::stream::pending::<Result<HostEvent, XyDriverError>>();
        run_interactive_bang(session, driver, bash, agent_stream, input).await?;
    }

    if let Some(stream) = agent_stream.as_mut() {
        while let Some(xy) = stream.next().await {
            session.step(HostEvent::Xy(Box::new(xy)))?;
        }
        *agent_stream = None;
        session.on_run_stream_closed();
        refresh_footer_tokens(session, driver).await;
        let _ = session.render_now();
    }

    if session.should_quit() {
        session.tui.finish();
    }
    Ok(())
}

pub async fn pump_idle_downlink<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
) -> Result<(), XyDriverError> {
    for ev in driver.drain_idle_events() {
        session.step(HostEvent::Xy(Box::new(ev)))?;
    }
    let _ = session.render_now();
    Ok(())
}

mod samples;
pub use samples::{harness_sample_message_history_tree, harness_sample_session_messages};

mod terminal;
#[cfg(test)]
pub(crate) use terminal::footer_line;
pub use terminal::{TestTerminal, alt_enter_event, enter_event, esc_event, key_event};

#[cfg(test)]
mod slice_tests;
