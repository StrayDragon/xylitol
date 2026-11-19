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

#[cfg(test)]
mod tests {
    use super::drain_pending_ui;
    use super::theme;
    use crate::bridge::UiEntry;
    use crate::dispatch::{DispatchOutcome, SessionCommandExecutor, transport_variant_error};
    use crate::driver::{
        CommandInfo, EventStream, LoadedResourcesSnapshot, McpServerPhase, McpServerSnapshot,
        ModelInfo, XyDriver, XyDriverError,
    };
    use crate::harness::{ScriptedDriver, TestTerminal};
    use crate::host::HostSession;
    use async_trait::async_trait;
    use xylitol_plugin_sdk::mcp::McpOAuthFlow;
    use xylitol_plugin_sdk::model::ContextTokenEstimate;
    use xylitol_protocol::Command;

    fn product_session() -> HostSession<TestTerminal> {
        HostSession::new_product_ui(TestTerminal::new(80, 24))
    }

    fn scroll_notices(session: &HostSession<TestTerminal>) -> Vec<String> {
        session
            .ui_model()
            .entries
            .iter()
            .filter_map(|entry| match entry {
                UiEntry::ScrollNotice { text } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    fn mount_mcp_server_panel(session: &mut HostSession<TestTerminal>) {
        let root = session.ui_root().expect("ui root").clone();
        let snap = LoadedResourcesSnapshot {
            mcp_configured: 1,
            mcp_servers: vec![McpServerSnapshot {
                id: "srv".into(),
                phase: McpServerPhase::Connected,
                tools_armed: true,
                tool_count: 1,
            }],
            ..LoadedResourcesSnapshot::default()
        };
        root.borrow_mut().mount_mcp_panel(&snap);
    }

    struct OAuthDriver {
        flow: Option<McpOAuthFlow>,
        oauth_starts: Vec<String>,
    }

    #[async_trait]
    impl XyDriver for OAuthDriver {
        async fn run(&mut self, _prompt: &str) -> EventStream {
            Box::pin(futures::stream::iter(Vec::new()))
        }

        fn abort(&self) {}

        fn current_model(&self) -> Option<ModelInfo> {
            None
        }

        fn available_models(&self) -> Vec<ModelInfo> {
            Vec::new()
        }

        fn thinking_level(&self) -> String {
            "off".into()
        }

        fn session_id(&self) -> Option<String> {
            Some("s-oauth".into())
        }

        fn get_commands(&self) -> Vec<CommandInfo> {
            Vec::new()
        }

        async fn estimate_context_tokens(&self) -> Result<ContextTokenEstimate, XyDriverError> {
            Err(XyDriverError::unsupported("estimate"))
        }

        fn leaf_entry_id(&self) -> Option<String> {
            None
        }

        async fn loaded_resources_snapshot(&self) -> LoadedResourcesSnapshot {
            LoadedResourcesSnapshot::default()
        }

        async fn mcp_oauth_start(&mut self, server: &str) -> Result<McpOAuthFlow, XyDriverError> {
            self.oauth_starts.push(server.to_string());
            match self.flow.clone() {
                Some(flow) => Ok(flow),
                None => Err(XyDriverError::message("oauth unavailable")),
            }
        }
    }

    #[async_trait]
    impl SessionCommandExecutor for OAuthDriver {
        async fn execute_session_command(
            &mut self,
            cmd: Command,
        ) -> Result<DispatchOutcome, XyDriverError> {
            Err(transport_variant_error(&cmd))
        }
    }

    #[tokio::test]
    async fn session_sidebar_intent_shows_sidebar_at_standard_width() {
        let mut session = product_session();
        let root = session.ui_root().expect("ui root").clone();
        root.borrow_mut().request_session_sidebar();
        let mut driver = ScriptedDriver::new();
        drain_pending_ui(&mut session, &mut driver).await;
        assert_eq!(root.borrow().session_sidebar_reserved_cols(80), 24);
        assert!(!root.borrow().has_pending_session_sidebar());
    }

    #[tokio::test]
    async fn tree_fork_failure_stops_remaining_intent_drain() {
        let mut session = product_session();
        let root = session.ui_root().expect("ui root").clone();
        session.request_session_tree_fork("e1");
        root.borrow_mut().request_session_sidebar();
        let mut driver = ScriptedDriver::new();
        driver.set_get_messages_error(Some("boom".into()));
        drain_pending_ui(&mut session, &mut driver).await;
        assert!(root.borrow().has_pending_session_sidebar());
        assert!(
            scroll_notices(&session)
                .iter()
                .any(|notice| notice.contains("fork: get_messages failed"))
        );
    }

    #[tokio::test]
    async fn mcp_oauth_start_success_publishes_url_and_closes_slot() {
        let mut session = product_session();
        mount_mcp_server_panel(&mut session);
        let root = session.ui_root().expect("ui root").clone();
        assert!(root.borrow().mcp_open());
        let mut driver = OAuthDriver {
            flow: Some(McpOAuthFlow {
                flow_id: "f1".into(),
                server: "srv".into(),
                auth_url: "https://auth.example/start".into(),
                redirect_uri: "http://127.0.0.1:1/cb".into(),
                scopes: Vec::new(),
                expires_at_ms: 1_700_000_000_000,
            }),
            oauth_starts: Vec::new(),
        };
        super::super::mcp::oauth_start(&mut session, &mut driver, "srv".into()).await;
        assert_eq!(driver.oauth_starts, vec!["srv".to_string()]);
        assert!(scroll_notices(&session).iter().any(|notice| {
            notice.contains("MCP srv authorization URL: https://auth.example/start")
        }));
        assert!(!root.borrow().mcp_open());
    }

    #[tokio::test]
    async fn mcp_oauth_start_failure_notices_in_scroll() {
        let mut session = product_session();
        mount_mcp_server_panel(&mut session);
        let mut driver = OAuthDriver {
            flow: None,
            oauth_starts: Vec::new(),
        };
        super::super::mcp::oauth_start(&mut session, &mut driver, "srv".into()).await;
        assert_eq!(driver.oauth_starts, vec!["srv".to_string()]);
        assert!(
            scroll_notices(&session)
                .iter()
                .any(|notice| notice.contains("mcp oauth start failed: srv"))
        );
    }

    #[tokio::test]
    async fn theme_select_unknown_name_notes_scroll_error() {
        let mut session = product_session();
        theme::select(&mut session, "definitely-missing".into()).await;
        assert_eq!(session.theme_preference(), None);
        assert!(
            scroll_notices(&session)
                .iter()
                .any(|notice| notice.contains("/theme failed"))
        );
    }

    #[tokio::test]
    async fn theme_select_builtin_light_applies_preference() {
        let mut session = product_session();
        theme::select(&mut session, "light".into()).await;
        assert_eq!(session.theme_preference(), Some("light"));
        assert!(scroll_notices(&session).is_empty());
    }
}
