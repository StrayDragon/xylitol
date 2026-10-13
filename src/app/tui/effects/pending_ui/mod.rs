//! Non-slash pending UI ops drained after slash (tree / pickers) (c1170).

mod clipboard;
mod import;
mod models;
mod resume;
mod theme;
mod tree;

use xylitol_tui::Terminal;

use crate::app::core::driver::XyDriver;
use crate::app::tui::host::HostSession;

pub(super) async fn drain_pending_ui<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
) {
    // Ask tool: mount Choice / complete oneshot before other UI pending (c1850).
    session.poll_ask_host();

    clipboard::take_paste_image(session, driver).await;
    // c2875 会话详情抽屉：pending 打开请求 → 身份供给后上屏。
    if session.take_pending_session_drawer() {
        let id = driver.session_id().unwrap_or_default();
        session.show_session_drawer(&id);
    }
    tree::open(session, driver).await;
    tree::travel(session, driver).await;
    if let Some(entry_id) = session.take_pending_session_tree_fork() {
        // Original pump aborted remaining UI ops if parent get_messages failed.
        if !tree::fork(session, driver, entry_id).await {
            return;
        }
    }
    tree::label(session, driver).await;
    models::select(session, driver).await;
    theme::select(session).await;
    import::decide(session, driver).await;
    resume::select(session, driver).await;
    resume::rename(session, driver).await;
    resume::delete(session, driver).await;
}
