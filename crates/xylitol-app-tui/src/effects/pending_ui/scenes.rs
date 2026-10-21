use xylitol_tui::Terminal;

use crate::driver::XyDriver;
use crate::effects::designing;
use crate::host::HostSession;

pub(super) async fn select<T: Terminal>(session: &mut HostSession<T>, driver: &mut dyn XyDriver) {
    let Some(scene) = session.take_pending_scene_select() else {
        return;
    };
    session.close_scenes_slot();
    designing::inject_scene(session, driver, scene).await;
}
