mod capabilities;
mod clipboard;
mod history;
mod import;
mod models;
mod resume;
mod scenes;
mod theme;
mod tree;

use xylitol_tui::Terminal;

use crate::driver::XyDriver;
use crate::host::HostSession;

pub(super) async fn drain_pending_ui<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
) {
    session.poll_ask_host();

    clipboard::take_paste_image(session, driver).await;
    if session.take_pending_session_popup() {
        let id = driver.session_id().unwrap_or_default();
        session.show_session_popup(&id);
    }
    if session.take_pending_session_sidebar() {
        let id = driver.session_id().unwrap_or_default();
        if session.session_sidebar_available() {
            session.show_session_sidebar(&id);
        } else {
            session.show_session_popup(&id);
        }
    }
    tree::open(session, driver).await;
    tree::travel(session, driver).await;
    if let Some(entry_id) = session.take_pending_session_tree_fork()
        && !tree::fork(session, driver, entry_id).await
    {
        return;
    }
    tree::label(session, driver).await;
    models::select(session, driver).await;
    scenes::select(session, driver).await;
    theme::select(session).await;
    capabilities::apply(session, driver).await;
    import::decide(session, driver).await;
    history::load_earlier(session, driver).await;
    resume::select(session, driver).await;
    resume::rename(session, driver).await;
    resume::delete(session, driver).await;
}
