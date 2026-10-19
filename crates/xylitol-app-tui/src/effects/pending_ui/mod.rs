//! Non-slash pending UI ops drained after slash (tree / pickers) (c1170).

mod clipboard;
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
    // Ask tool: mount Choice / complete oneshot before other UI pending (c1850).
    session.poll_ask_host();

    clipboard::take_paste_image(session, driver).await;
    // c2875 会话详情覆盖弹出框：pending 打开请求 → 身份供给后上屏。
    if session.take_pending_session_popup() {
        let id = driver.session_id().unwrap_or_default();
        session.show_session_popup(&id);
    }
    // c2905/c2918 会话侧栏：同握手，身份共享；标准档以下侧栏不可用——
    // 降级打开覆盖弹出框（信息不丢、形态切换，Esc 收起走 c2914）。
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
    if let Some(entry_id) = session.take_pending_session_tree_fork() {
        // Original pump aborted remaining UI ops if parent get_messages failed.
        if !tree::fork(session, driver, entry_id).await {
            return;
        }
    }
    tree::label(session, driver).await;
    models::select(session, driver).await;
    scenes::select(session, driver).await;
    theme::select(session).await;
    import::decide(session, driver).await;
    resume::select(session, driver).await;
    resume::rename(session, driver).await;
    resume::delete(session, driver).await;
}
