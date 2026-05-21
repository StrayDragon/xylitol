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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::r#loop::AgentEvent;
    use serde_json::json;

    #[tokio::test]
    async fn faux_provider_tracks_calls_and_supports_tool_calls() {
        let provider = faux_provider::FauxProvider::new("faux");
        provider.set_responses(vec![
            faux_provider::FauxResponseStep::tool_call("read", json!({"file_path": "README.md"})),
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
    }

    #[tokio::test]
    async fn harness_runs_prompt_and_provider_captures_request_contents() {
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
                .flat_map(|r| r.contents.iter())
                .any(|c| c.role == "user"),
            "expected at least one user content in captured requests"
        );
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

    #[cfg(feature = "dev-vt100")]
    #[test]
    fn vt100_chat_renders_thinking_and_tool_calls_flat() {
        use ratatui::Terminal;
        use serde_json::json;

        use crate::interface::tui::{ChatComponent, Component, MarkdownRenderer, TuiEvent};

        let backend = vt100_backend::VT100Backend::new(/*width*/ 70, /*height*/ 18);
        let mut terminal = Terminal::new(backend).expect("terminal");

        let mut chat = ChatComponent::new(MarkdownRenderer::default());
        chat.add_user_message("Please find the function definition.");

        let _ = chat.handle_event(&TuiEvent::Agent(AgentEvent::ThinkingDelta(
            "I'll search in app.rs then summarize.".to_string(),
        )));
        let _ = chat.handle_event(&TuiEvent::Agent(AgentEvent::ToolCallStart {
            id: "call-1".to_string(),
            name: "search".to_string(),
            args: json!({"query": "fn copy_to_clipboard", "path": "src/interface/tui/app.rs"}),
        }));
        let _ = chat.handle_event(&TuiEvent::Agent(AgentEvent::ToolCallEnd {
            id: "call-1".to_string(),
            result: json!({"ok": true}),
        }));
        let _ = chat.handle_event(&TuiEvent::Agent(AgentEvent::TextDelta(
            "Found it in `src/interface/tui/app.rs`.".to_string(),
        )));
        let _ = chat.handle_event(&TuiEvent::Agent(AgentEvent::StepComplete {
            step: 1,
            summary: String::new(),
        }));

        terminal
            .draw(|frame| chat.render(frame, frame.area()))
            .expect("draw");

        let screen = terminal.backend().vt100().screen().contents();
        insta::assert_snapshot!("vt100_chat_thinking_tool_flat", screen);
    }
}
