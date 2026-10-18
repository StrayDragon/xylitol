//! Designing preview inject (process-local; not a product slash).
//!
//! c2740/c2930: scenes are seeded into an in-memory store and applied to the
//! transcript directly; no `protocol::Command` / remote unary.
//! **designing preview** = 把命名场景注入同一 TUI 画布的预览设计。

use xylitol_tui::Terminal;

use crate::app::core::driver::{DebugSceneLoad, XyDriver};
use crate::app::debug_fixtures::PreviewInject;
use crate::app::tui::host::HostSession;
use crate::protocol::ports::XySessionStore;

pub(crate) async fn inject_scene<T: Terminal>(
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
                super::debug_verify::run_verify_smoke(session, driver).await;
            }
            (PreviewInject::FixedZone(op), _) => {
                session.apply_fixed_zone_op(op);
            }
            (PreviewInject::LiveTape, "activity-fold-live") => {
                super::debug_activity_fold::run_activity_fold_live(session);
            }
            (PreviewInject::LiveXy, _) => {
                super::debug_activity_fold::run_preview_live_xy(session);
            }
            (PreviewInject::Resume, _) => match load_and_bind_scene(driver, meta.id).await {
                Ok(load) => session.apply_debug_scene(load),
                Err(e) => note_load_err(session, "tui.designing_preview", &e, e.to_string()),
            },
            (inject, id) => {
                session.push_scroll_notice(format!(
                    "{id}: inject {inject:?} has no designing preview runner"
                ));
            }
        }
    } else {
        match load_and_bind_scene(driver, &scene).await {
            Ok(load) => session.apply_debug_scene(load),
            Err(e) => note_load_err(session, "tui.designing_preview", &e, e.to_string()),
        }
    }
    let _ = session.render_now();
}

/// Seed a scene into the driver's store when the Host is in-process (designing
/// uses an ephemeral store). Otherwise a throwaway in-memory manager — never
/// the caller's current JSONL file, never the RPC surface.
async fn load_and_bind_scene(
    driver: &mut dyn XyDriver,
    scene: &str,
) -> Result<DebugSceneLoad, crate::app::core::driver_error::XyDriverError> {
    use crate::app::core::driver_error::XyDriverError;

    let short = &uuid::Uuid::new_v4().to_string()[..8];
    let canonical = crate::app::debug_fixtures::resolve_scene_id(scene).ok_or_else(|| {
        XyDriverError::invalid_input(format!(
            "unknown designing scene: {scene}\n{}",
            crate::app::debug_fixtures::list_note()
        ))
    })?;
    let session_id = format!("designing-{canonical}-{short}");
    let owned = crate::infra::session::SessionManager::in_memory();
    let bound = driver.session_store();
    let store: &dyn XySessionStore = match bound.as_ref() {
        Some(s) => s.as_ref(),
        None => &owned,
    };
    store
        .create(&session_id, None, None)
        .await
        .map_err(XyDriverError::from)?;
    let canonical = crate::app::debug_fixtures::seed_scene(store, &session_id, scene).await?;
    let entries = store.load_entries(&session_id).await?;
    if bound.is_some() {
        let _ = crate::app::core::dispatch::dispatch(
            driver,
            crate::protocol::Command::SwitchSession {
                session_path: session_id.clone(),
            },
        )
        .await;
    }
    Ok(DebugSceneLoad {
        note: format!("designing preview `{canonical}` → local in-memory session {session_id}"),
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
    session.push_toast_notice(text);
}
