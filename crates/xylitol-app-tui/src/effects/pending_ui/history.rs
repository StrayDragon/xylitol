use xylitol_tui::Terminal;

use crate::driver::XyDriver;
use crate::history_window::HISTORY_PAGE;
use crate::host::HostSession;

pub(super) async fn load_earlier<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
) {
    if !session.take_load_earlier_request() {
        return;
    }
    let Some(oldest) = session.oldest_loaded_entry_id() else {
        return;
    };
    match super::super::session_entries_page(driver, Some(HISTORY_PAGE), Some(oldest)).await {
        Ok((older, has_more)) => {
            if older.is_empty() {
                session.clear_earlier_hint();
                session.sync_ui_root_from_model();
                return;
            }
            session.prepend_history(older, has_more);
        }
        Err(e) => {
            super::super::helpers::note_driver_err(
                session,
                "tui.history.load_earlier",
                &e,
                format!("load earlier: get_messages failed: {e}"),
            );
        }
    }
}
