//! Designing scene catalog confirm → process-local designing preview inject.
//!
//! Confirm closes the catalog so the canvas and editor return. Reopen with
//! `/designing` (designing composition only).

use xylitol_tui::Terminal;

use crate::app::core::driver::XyDriver;
use crate::app::tui::effects::designing;
use crate::app::tui::host::HostSession;

pub(super) async fn select<T: Terminal>(session: &mut HostSession<T>, driver: &mut dyn XyDriver) {
    let Some(scene) = session.take_pending_scene_select() else {
        return;
    };
    session.close_scenes_slot();
    designing::inject_scene(session, driver, scene).await;
}
