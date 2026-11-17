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
use crate::layout::UiIntent;

pub(super) async fn drain_pending_ui<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
) {
    session.drain_mcp_completion_picks();
    session.poll_ask_host();

    clipboard::take_paste_image(session, driver).await;
    super::mcp::prompt_args(session, driver).await;
    while let Some(intent) = session.pop_ui_intent() {
        match intent {
            UiIntent::SessionSidebar => {
                let id = driver.session_id().unwrap_or_default();
                if session.session_sidebar_available() {
                    session.show_session_sidebar(&id);
                }
            }
            UiIntent::TreeOpen => tree::open(session, driver).await,
            UiIntent::TreeTravel(entry_id) => tree::travel(session, driver, entry_id).await,
            UiIntent::TreeFork(entry_id) => {
                if !tree::fork(session, driver, entry_id).await {
                    return;
                }
            }
            UiIntent::TreeLabel(entry_id, label) => {
                tree::label(session, driver, entry_id, label).await;
            }
            UiIntent::ModelSelect(choice) => models::select(session, driver, choice).await,
            UiIntent::SceneSelect(scene) => scenes::select(session, driver, scene).await,
            UiIntent::ThemeSelect(theme_name) => theme::select(session, theme_name).await,
            UiIntent::CapabilityToggle(id, enabled) => {
                capabilities::apply(session, driver, id, enabled).await;
            }
            UiIntent::ImportDecision(decision) => import::decide(session, driver, decision).await,
            UiIntent::SessionResumeSelect(session_id) => {
                resume::select(session, driver, session_id).await;
            }
            UiIntent::SessionResumeRename(id, name) => {
                resume::rename(session, driver, id, name).await;
            }
            UiIntent::SessionResumeDelete(id) => resume::delete(session, driver, id).await,
            UiIntent::McpPromptSelect(server, name) => {
                super::mcp::prompt_select(session, driver, server, name).await;
            }
            UiIntent::McpOAuthStart(server) => {
                super::mcp::oauth_start(session, driver, server).await;
            }
            UiIntent::McpResourceAdd(resource) => {
                super::mcp::resource_add(session, resource).await;
            }
        }
    }
    history::load_earlier(session, driver).await;
}
