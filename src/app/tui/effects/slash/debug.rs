//! `/debug` scene inject arm.

use xylitol_tui::Terminal;

use crate::app::core::driver::XyDriver;
use crate::app::debug_fixtures::PreviewInject;
use crate::app::tui::host::HostSession;

use super::super::helpers::note_driver_err;

pub(super) async fn run<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
    scene: String,
) {
    let scene = scene.trim().to_ascii_lowercase();
    if scene.is_empty() || scene == "list" {
        session.push_scroll_notice(crate::app::debug_fixtures::list_note());
    } else if let Some(meta) = crate::app::debug_fixtures::find_scene(&scene) {
        match (meta.inject, meta.id) {
            (
                PreviewInject::FixedZone(crate::app::debug_fixtures::FixedZoneOp::SlotModels),
                "verify-smoke",
            ) => {
                super::super::debug_verify::run_verify_smoke(session, driver).await;
            }
            (PreviewInject::FixedZone(op), _) => {
                session.apply_fixed_zone_op(op);
            }
            (PreviewInject::LiveTape, "activity-fold-live") => {
                super::super::debug_activity_fold::run_activity_fold_live(session);
            }
            (PreviewInject::LiveXy, _) => {
                super::super::debug_activity_fold::run_preview_live_xy(session);
            }
            (PreviewInject::Resume, _) => {
                log::info!(target: "xylitol::tui", "XyDriver::load_debug_scene scene={}", meta.id);
                match driver.load_debug_scene(meta.id).await {
                    Ok(load) => session.apply_debug_scene(load),
                    Err(e) => note_driver_err(session, "tui.load_debug_scene", &e, e.to_string()),
                }
            }
            (inject, id) => {
                session.push_scroll_notice(format!(
                    "{id}: inject {inject:?} has no /debug runner yet"
                ));
            }
        }
    } else {
        log::info!(target: "xylitol::tui", "XyDriver::load_debug_scene scene={}", scene);
        match driver.load_debug_scene(&scene).await {
            Ok(load) => session.apply_debug_scene(load),
            Err(e) => note_driver_err(session, "tui.load_debug_scene", &e, e.to_string()),
        }
    }
    let _ = session.render_now();
}
