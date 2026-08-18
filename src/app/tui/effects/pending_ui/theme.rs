//! Themes picker confirm → reload_themes.

use xylitol_tui::Terminal;

use crate::app::tui::host::HostSession;

use super::super::helpers::note_driver_err;

pub(super) async fn select<T: Terminal>(session: &mut HostSession<T>) {
    let Some(theme_name) = session.take_pending_theme_select() else {
        return;
    };
    log::info!(target: "xylitol::tui", "reload_themes from picker theme={}", theme_name);
    match session.reload_themes(&theme_name) {
        Ok(()) => {
            session.close_themes_slot();
        }
        Err(e) => note_driver_err(
            session,
            "tui.reload_themes.picker",
            &e,
            format!("/theme failed: {e}"),
        ),
    }
    let _ = session.render_now();
}
