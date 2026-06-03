//! Shared test infrastructure (single-crate replacement for a dedicated test-support crate).

pub(crate) mod faux_provider;
pub(crate) mod harness;
pub(crate) mod in_memory;
pub(crate) mod sse_mock;

#[cfg(feature = "dev-vt100")]
pub(crate) mod vt100_backend;

#[cfg(test)]
#[ctor::ctor]
fn init_insta_workspace_root() {
    // Safety: tests are single-process and this is set once at startup.
    unsafe {
        std::env::set_var("INSTA_WORKSPACE_ROOT", env!("CARGO_MANIFEST_DIR"));
    }
}

/// Workspace root path for tests that need to reference project files.
#[cfg(test)]
pub(crate) fn workspace_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Wrap an async test future with a timeout to prevent CI hangs.
/// Default timeout is 10 seconds; override via the `secs` parameter.
#[cfg(test)]
pub(crate) async fn with_test_timeout<F, T>(secs: u64, future: F) -> T
where
    F: std::future::Future<Output = T>,
{
    tokio::time::timeout(std::time::Duration::from_secs(secs), future)
        .await
        .expect("test timed out")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::r#loop::AgentEvent;
    use serde_json::json;

    #[tokio::test]
    async fn faux_provider_tracks_calls_and_supports_tool_calls() {
        super::with_test_timeout(10, async {
            let provider = faux_provider::FauxProvider::new("faux");
            provider.set_responses(vec![
                faux_provider::FauxResponseStep::tool_call(
                    "read",
                    json!({"file_path": "README.md"}),
                ),
                faux_provider::FauxResponseStep::text("done"),
            ]);

            let mut harness = harness::TestHarness::builder()
                .with_model(provider.clone_box())
                .with_tools(crate::agent::tools::ToolRegistry::builtins())
                .build()
                .await;

            let events = harness.run("test", "test-session").await;
            assert!(provider.call_count() >= 2, "expected >=2 model calls");
            assert!(
                events
                    .iter()
                    .any(|e| matches!(e, AgentEvent::ToolCallStart { name, .. } if name == "read")),
                "expected ToolCallStart"
            );
            assert!(
                events
                    .iter()
                    .any(|e| matches!(e, AgentEvent::ToolCallEnd { .. })),
                "expected ToolCallEnd"
            );
            assert!(
                events
                    .iter()
                    .any(|e| matches!(e, AgentEvent::TextDelta(t) if t.contains("done"))),
                "expected assistant text"
            );
        })
        .await;
    }

    #[tokio::test]
    async fn harness_runs_prompt_and_provider_captures_request_contents() {
        super::with_test_timeout(10, async {
            let provider = faux_provider::FauxProvider::new("faux");
            provider.set_responses(vec![faux_provider::FauxResponseStep::text("ok")]);

            let mut harness = harness::TestHarness::builder()
                .with_model(provider.clone_box())
                .build()
                .await;

            let _events = harness.run("hello", "test-session-2").await;
            let requests = provider.captured_requests();
            assert!(
                requests
                    .iter()
                    .flatten()
                    .any(|c| c.role == crate::agent::types::XyRole::User),
                "expected at least one user content in captured requests"
            );
        })
        .await;
    }

    #[cfg(feature = "dev-vt100")]
    #[test]
    fn vt100_backend_renders_widget_contents() {
        use ratatui::Terminal;
        use ratatui::widgets::Paragraph;

        let backend = vt100_backend::VT100Backend::new(/*width*/ 20, /*height*/ 6);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal
            .draw(|frame| {
                frame.render_widget(Paragraph::new("Hello"), frame.area());
            })
            .expect("draw");

        let screen = terminal.backend().vt100().screen().contents();
        insta::assert_snapshot!("vt100_widget_hello", screen);
    }
}
