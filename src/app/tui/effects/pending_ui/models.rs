//! Models picker confirm → SetModel + thinking level.

use xylitol_tui::Terminal;

use crate::app::core::dispatch::{DispatchOutcome, dispatch};
use crate::app::core::driver::XyDriver;
use crate::app::tui::host::HostSession;
use crate::protocol::Command;

pub(super) async fn select<T: Terminal>(session: &mut HostSession<T>, driver: &mut dyn XyDriver) {
    let Some(choice) = session.take_pending_model_select() else {
        return;
    };
    log::info!(target: "xylitol::tui", "SetModel from picker model_id={}", choice.model_id);
    match dispatch(
        driver,
        Command::SetModel {
            provider: String::new(),
            model_id: choice.model_id.clone(),
        },
    )
    .await
    {
        Ok(DispatchOutcome::Model(_)) => {
            if let Err(e) = dispatch(
                driver,
                Command::SetThinkingLevel {
                    level: choice.thinking.clone(),
                },
            )
            .await
            {
                e.log_failure("tui.set_thinking_level");
                session.push_scroll_notice(format!("thinking level failed: {e}"));
            }
            session.sync_fixed_zone(driver);
            session.close_models_slot();
        }
        Ok(_) => {
            let _ = dispatch(
                driver,
                Command::SetThinkingLevel {
                    level: choice.thinking.clone(),
                },
            )
            .await;
            session.sync_fixed_zone(driver);
            session.close_models_slot();
        }
        // dispatch already logs error.kind
        Err(e) => session.push_scroll_notice(format!("/model failed: {e}")),
    }
    let _ = session.render_now();
}
