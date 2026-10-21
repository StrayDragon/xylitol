use xylitol_tui::Terminal;

use crate::dispatch::{DispatchOutcome, dispatch};
use crate::driver::XyDriver;
use crate::host::HostSession;
use xylitol_protocol::Command;

pub(super) async fn open<T: Terminal>(session: &mut HostSession<T>, driver: &mut dyn XyDriver) {
    match dispatch(driver, Command::GetAvailableModels {}).await {
        Ok(DispatchOutcome::Models(models)) => {
            let current = driver.current_model().map(|m| m.id);
            session.mount_models_picker(models, current, driver.thinking_level());
        }
        Ok(_) => session.push_scroll_notice("models list unavailable"),
        Err(e) => session.push_scroll_notice(format!("/model failed: {e}")),
    }
    let _ = session.render_now();
    if !session.is_busy() {
        let _ = driver.refresh_surface_caches().await;
    }
}

pub(super) async fn set<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
    model_id: String,
) {
    match dispatch(
        driver,
        Command::SetModel {
            provider: String::new(),
            model_id,
        },
    )
    .await
    {
        Ok(DispatchOutcome::Model(_)) => {
            session.sync_fixed_zone(driver);
        }
        Ok(_) => {
            session.sync_fixed_zone(driver);
        }
        Err(e) => session.push_scroll_notice(format!("/model failed: {e}")),
    }
    let _ = session.render_now();
}
