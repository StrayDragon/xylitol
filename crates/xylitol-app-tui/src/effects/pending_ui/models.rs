use xylitol_tui::Terminal;

use crate::dispatch::{DispatchOutcome, dispatch};
use crate::driver::XyDriver;
use crate::host::HostSession;
use crate::layout::PendingModelChoice;
use xylitol_protocol::Command;

pub(super) async fn select<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
    choice: PendingModelChoice,
) {
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
        Err(e) => session.push_scroll_notice(format!("/model failed: {e}")),
    }
    let _ = session.render_now();
}
