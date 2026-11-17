use xylitol_tui::Terminal;

use crate::driver::XyDriver;
use crate::effects::designing;
use crate::host::HostSession;

pub(super) async fn select<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
    scene: String,
) {
    session.close_scenes_slot();
    designing::inject_scene(session, driver, scene).await;
}
