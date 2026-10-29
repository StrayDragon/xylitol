use xylitol_tui::Terminal;

use crate::dispatch::{DispatchOutcome, dispatch};
use crate::driver::XyDriver;
use crate::host::HostSession;
use xylitol_protocol::Command;

use super::super::helpers::note_driver_err;

pub(super) async fn apply<T: Terminal>(session: &mut HostSession<T>, driver: &mut dyn XyDriver) {
    let Some((id, enabled)) = session.take_pending_capability_toggle() else {
        return;
    };
    log::info!(
        target: "xylitol::tui",
        "SetCapability from capabilities slot id={id} enabled={enabled}"
    );
    match dispatch(driver, Command::SetCapability { id, enabled }).await {
        Ok(DispatchOutcome::Capabilities(snap)) => {
            session.apply_capabilities_snapshot(&snap);
            session.sync_fixed_zone(driver);
        }
        Ok(_) => {}
        Err(e) => note_driver_err(
            session,
            "tui.set_capability",
            &e,
            format!("/capabilities failed: {e}"),
        ),
    }
    let _ = session.render_now();
}
