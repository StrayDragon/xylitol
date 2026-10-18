//! Preview scene catalog confirm → process-local inject.

use xylitol_tui::Terminal;

use crate::app::core::driver::XyDriver;
use crate::app::tui::effects::preview;
use crate::app::tui::host::HostSession;

pub(super) async fn select<T: Terminal>(session: &mut HostSession<T>, driver: &mut dyn XyDriver) {
    let Some(scene) = session.take_pending_scene_select() else {
        return;
    };
    session.close_scenes_slot();
    preview::inject_scene(session, driver, scene).await;
}
