//! `/debug` scene inject arm.
//!
//! c2740: scenes are process-local — seeded into an in-memory store and
//! applied to the transcript directly; `load_debug_scene` no longer exists on
//! the wire / Driver / Host surface.

use xylitol_tui::Terminal;

use crate::app::core::driver::{DebugSceneLoad, XyDriver};
use crate::app::debug_fixtures::PreviewInject;
use crate::app::tui::host::HostSession;
use crate::protocol::ports::XySessionStore;

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
            (PreviewInject::Resume, _) => match load_scene_in_memory(meta.id).await {
                Ok(load) => session.apply_debug_scene(load),
                Err(e) => note_load_err(session, "tui.debug_scene", &e, e.to_string()),
            },
            (inject, id) => {
                session.push_scroll_notice(format!(
                    "{id}: inject {inject:?} has no /debug runner yet"
                ));
            }
        }
    } else {
        // Alias resolution still applies (seed_scene resolves canonical ids).
        match load_scene_in_memory(&scene).await {
            Ok(load) => session.apply_debug_scene(load),
            Err(e) => note_load_err(session, "tui.debug_scene", &e, e.to_string()),
        }
    }
    let _ = session.render_now();
}

/// Seed a scene into an in-memory session and collect its entries (c2740):
/// never touches the user's session files, never crosses the RPC surface.
async fn load_scene_in_memory(
    scene: &str,
) -> Result<DebugSceneLoad, crate::app::core::driver_error::XyDriverError> {
    use crate::app::core::driver_error::XyDriverError;

    let short = &uuid::Uuid::new_v4().to_string()[..8];
    let canonical = crate::app::debug_fixtures::resolve_scene_id(scene).ok_or_else(|| {
        XyDriverError::invalid_input(format!(
            "unknown debug scene: {scene}\n{}",
            crate::app::debug_fixtures::list_note()
        ))
    })?;
    let session_id = format!("debug-{canonical}-{short}");
    let mgr = crate::infra::session::SessionManager::in_memory();
    mgr.create(&session_id, None, None)
        .await
        .map_err(XyDriverError::from)?;
    let canonical = crate::app::debug_fixtures::seed_scene(&mgr, &session_id, scene).await?;
    let entries = mgr.load_entries(&session_id).await?;
    Ok(DebugSceneLoad {
        note: format!("debug scene `{canonical}` → local in-memory session {session_id}"),
        session_id,
        entries,
        model: None,
    })
}

fn note_load_err<T: Terminal>(
    session: &mut HostSession<T>,
    target: &str,
    err: &crate::app::core::driver_error::XyDriverError,
    text: String,
) {
    log::warn!(target: "xylitol::tui", "{target} failed: {err}");
    session.push_scroll_notice(format!("/debug failed: {text}"));
}
