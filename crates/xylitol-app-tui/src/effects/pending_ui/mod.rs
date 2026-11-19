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
    drain_intents(session, driver).await;
    history::load_earlier(session, driver).await;
}

async fn drain_intents<T: Terminal>(session: &mut HostSession<T>, driver: &mut dyn XyDriver) {
    while let Some(intent) = session.pop_ui_intent() {
        match intent {
            UiIntent::SessionSidebar => show_session_sidebar_intent(session, driver),
            UiIntent::TreeOpen
            | UiIntent::TreeTravel(_)
            | UiIntent::TreeFork(_)
            | UiIntent::TreeLabel(_, _) => {
                if tree::apply(session, driver, intent).await.is_break() {
                    return;
                }
            }
            UiIntent::SessionResumeSelect(_)
            | UiIntent::SessionResumeRename(_, _)
            | UiIntent::SessionResumeDelete(_) => {
                resume::apply(session, driver, intent).await;
            }
            UiIntent::ModelSelect(_)
            | UiIntent::SceneSelect(_)
            | UiIntent::ThemeSelect(_)
            | UiIntent::CapabilityToggle(_, _)
            | UiIntent::ImportDecision(_) => {
                apply_picker_intent(session, driver, intent).await;
            }
            UiIntent::McpPromptSelect(_, _)
            | UiIntent::McpOAuthStart(_)
            | UiIntent::McpResourceAdd(_) => {
                super::mcp::apply(session, driver, intent).await;
            }
        }
    }
}

fn show_session_sidebar_intent<T: Terminal>(session: &mut HostSession<T>, driver: &dyn XyDriver) {
    let id = driver.session_id().unwrap_or_default();
    if session.session_sidebar_available() {
        session.show_session_sidebar(&id);
    }
}

async fn apply_picker_intent<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
    intent: UiIntent,
) {
    match intent {
        UiIntent::ModelSelect(choice) => models::select(session, driver, choice).await,
        UiIntent::SceneSelect(scene) => scenes::select(session, driver, scene).await,
        UiIntent::ThemeSelect(theme_name) => theme::select(session, theme_name).await,
        UiIntent::CapabilityToggle(id, enabled) => {
            capabilities::apply(session, driver, id, enabled).await;
        }
        UiIntent::ImportDecision(decision) => import::decide(session, driver, decision).await,
        _ => {}
    }
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
    async fn drain_routes_tree_intents_through_domain_apply() {
        let mut session = product_session();
        let root = session.ui_root().expect("ui root").clone();
        root.borrow_mut()
            .request_intent(crate::layout::UiIntent::TreeOpen);
        root.borrow_mut()
            .request_intent(crate::layout::UiIntent::TreeTravel("t1".into()));
        root.borrow_mut()
            .request_intent(crate::layout::UiIntent::TreeLabel(
                "t1".into(),
                Some("lbl".into()),
            ));
        let mut driver = ScriptedDriver::new();
        drain_pending_ui(&mut session, &mut driver).await;
        assert!(scroll_notices(&session).len() < 6);
    }

    #[tokio::test]
    async fn drain_routes_resume_intents_through_domain_apply() {
        let mut session = product_session();
        let root = session.ui_root().expect("ui root").clone();
        root.borrow_mut()
            .request_intent(crate::layout::UiIntent::SessionResumeSelect("s1".into()));
        root.borrow_mut()
            .request_intent(crate::layout::UiIntent::SessionResumeRename(
                "s1".into(),
                "renamed".into(),
            ));
        root.borrow_mut()
            .request_intent(crate::layout::UiIntent::SessionResumeDelete("s2".into()));
        let mut driver = ScriptedDriver::new();
        drain_pending_ui(&mut session, &mut driver).await;
    }

    #[tokio::test]
    async fn drain_routes_picker_intents_through_domain_apply() {
        let mut session = product_session();
        let root = session.ui_root().expect("ui root").clone();
        root.borrow_mut()
            .request_intent(crate::layout::UiIntent::ModelSelect(
                crate::layout::PendingModelChoice {
                    model_id: "m1".into(),
                    thinking: "off".into(),
                },
            ));
        root.borrow_mut()
            .request_intent(crate::layout::UiIntent::SceneSelect("coding".into()));
        root.borrow_mut()
            .request_intent(crate::layout::UiIntent::CapabilityToggle(
                "tools".into(),
                true,
            ));
        root.borrow_mut()
            .request_intent(crate::layout::UiIntent::ImportDecision(
                crate::layout::ImportConfirmDecision::Rejected,
            ));
        let mut driver = ScriptedDriver::new();
        drain_pending_ui(&mut session, &mut driver).await;
    }

    #[tokio::test]
    async fn drain_routes_mcp_intents_through_domain_apply() {
        let mut session = product_session();
        let root = session.ui_root().expect("ui root").clone();
        root.borrow_mut()
            .request_intent(crate::layout::UiIntent::McpPromptSelect(
                "srv".into(),
                "prompt1".into(),
            ));
        root.borrow_mut()
            .request_intent(crate::layout::UiIntent::McpResourceAdd(
                xylitol_plugin_sdk::mcp::McpResourceRef {
                    server: "srv".into(),
                    uri: "file:///tmp/x".into(),
                },
            ));
        let mut driver = ScriptedDriver::new();
        drain_pending_ui(&mut session, &mut driver).await;
    }

    #[tokio::test]
    async fn mcp_oauth_intent_in_queue_runs_oauth_start() {
        let mut session = product_session();
        let root = session.ui_root().expect("ui root").clone();
        root.borrow_mut()
            .request_intent(crate::layout::UiIntent::McpOAuthStart("srv".into()));
        let mut driver = OAuthDriver {
            flow: None,
            oauth_starts: Vec::new(),
        };
        drain_pending_ui(&mut session, &mut driver).await;
        assert_eq!(driver.oauth_starts, vec!["srv".to_string()]);
        assert!(scroll_notices(&session).iter().any(|n| n.contains("srv")));
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
