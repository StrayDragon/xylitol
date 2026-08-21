//! `/model` picker + SetModel slash arms.

use xylitol_tui::Terminal;

use crate::app::core::dispatch::{DispatchOutcome, dispatch};
use crate::app::core::driver::XyDriver;
use crate::app::tui::host::HostSession;
use crate::protocol::Command;

pub(super) async fn open<T: Terminal>(session: &mut HostSession<T>, driver: &mut dyn XyDriver) {
    // c1780: busy Allow — open list; selection still NextTurn via SetModel path.
    // Attach caches models asynchronously; never block the tick on HTTP.
    let _ = driver.refresh_surface_caches().await;
    match dispatch(driver, Command::GetAvailableModels { id: None }).await {
        Ok(DispatchOutcome::Models(models)) => {
            let current = driver.current_model().map(|m| m.id);
            session.mount_models_picker(models, current, driver.thinking_level());
        }
        Ok(_) => session.push_scroll_notice("models list unavailable"),
        Err(e) => session.push_scroll_notice(format!("/model failed: {e}")),
    }
    let _ = session.render_now();
}

pub(super) async fn set<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
    model_id: String,
) {
    match dispatch(
        driver,
        Command::SetModel {
            id: None,
            provider: String::new(),
            model_id,
        },
    )
    .await
    {
        Ok(DispatchOutcome::Model(_)) => {
            session.sync_runtime_chrome(driver);
        }
        Ok(_) => {
            session.sync_runtime_chrome(driver);
        }
        Err(e) => session.push_scroll_notice(format!("/model failed: {e}")),
    }
    let _ = session.render_now();
}
