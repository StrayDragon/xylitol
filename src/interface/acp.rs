//! ACP (Agent Client Protocol) mode — IDE integration via stdio.
//!
//! xylitol runs as an ACP agent subprocess spawned by an IDE (Zed, VS Code, etc.).
//! Uses the `agent-client-protocol` SDK for JSON-RPC 2.0 framing and transport.

use std::sync::Arc;

use agent_client_protocol::schema;
use agent_client_protocol::{
    ConnectionTo, Responder, Stdio, UntypedRole, on_receive_notification, on_receive_request,
};
use futures::StreamExt;

use crate::agent::r#loop::{AgentEvent, AgentLoop};
use crate::agent::repeat::DetectionConfig;
use crate::agent::tools::ToolRegistry;
use crate::infra::config::AppConfig;

/// Type alias for standard boxed-error results.
type BoxError = Box<dyn std::error::Error + Send + Sync>;

// ---------------------------------------------------------------------------
// Shared state
// ---------------------------------------------------------------------------

/// State shared across all ACP request handlers.
struct AcpState {
    config: AppConfig,
    tool_registry: ToolRegistry,
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Run the ACP mode: listen on stdio for ACP messages and handle them.
pub(crate) async fn run_acp_mode(
    config: AppConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let tool_registry = build_tool_registry(&config)
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { e.to_string().into() })?;
    let state = Arc::new(AcpState {
        config,
        tool_registry,
    });

    tracing::info!("ACP agent starting on stdio");

    let result = UntypedRole
        .builder()
        .name("xylitol")
        // ── initialize ────────────────────────────────────────────
        .on_receive_request(
            {
                let state = state.clone();
                move |req: schema::InitializeRequest,
                      responder: Responder<schema::InitializeResponse>,
                      _cx: ConnectionTo<UntypedRole>| {
                    let _state = state.clone();
                    async move {
                        tracing::info!(
                            "initialize: client={:?}, version={:?}",
                            req.client_info,
                            req.protocol_version,
                        );
                        responder.respond(
                            schema::InitializeResponse::new(req.protocol_version)
                                .agent_info(schema::Implementation::new(
                                    "xylitol",
                                    env!("CARGO_PKG_VERSION"),
                                )),
                        )
                    }
                }
            },
            on_receive_request!(),
        )
        // ── session/new ───────────────────────────────────────────
        .on_receive_request(
            {
                let state = state.clone();
                move |req: schema::NewSessionRequest,
                      responder: Responder<schema::NewSessionResponse>,
                      _cx: ConnectionTo<UntypedRole>| {
                    let _state = state.clone();
                    async move {
                        tracing::info!("session/new: cwd={:?}", req.cwd);
                        let session_id = schema::SessionId::new(uuid::Uuid::new_v4().to_string());
                        responder.respond(schema::NewSessionResponse::new(session_id))
                    }
                }
            },
            on_receive_request!(),
        )
        // ── session/prompt ────────────────────────────────────────
        .on_receive_request(
            {
                let state = state.clone();
                move |req: schema::PromptRequest,
                      responder: Responder<schema::PromptResponse>,
                      cx: ConnectionTo<UntypedRole>| {
                    let state = state.clone();
                    async move { handle_prompt(req, responder, cx, &state).await }
                }
            },
            on_receive_request!(),
        )
        // ── session/close ─────────────────────────────────────────
        .on_receive_request(
            {
                let state = state.clone();
                move |req: schema::CloseSessionRequest,
                      responder: Responder<schema::CloseSessionResponse>,
                      _cx: ConnectionTo<UntypedRole>| {
                    let _state = state.clone();
                    async move {
                        tracing::info!("session/close: session={:?}", req.session_id);
                        responder.respond(schema::CloseSessionResponse::new())
                    }
                }
            },
            on_receive_request!(),
        )
        // ── session/cancel (notification) ─────────────────────────
        .on_receive_notification(
            {
                let state = state.clone();
                move |notif: schema::CancelNotification,
                      _cx: ConnectionTo<UntypedRole>| {
                    let _state = state.clone();
                    async move {
                        tracing::info!("session/cancel: session={:?}", notif.session_id);
                        // TODO: wire cancellation token to abort running prompt
                        Ok(())
                    }
                }
            },
            on_receive_notification!(),
        )
        .connect_to(Stdio::new())
        .await;

    match result {
        Ok(()) => {
            tracing::info!("ACP agent disconnected");
            Ok(())
        }
        Err(e) => {
            tracing::error!("ACP agent error: {e}");
            Err(Box::<dyn std::error::Error + Send + Sync>::from(e))
        }
    }
}

// ---------------------------------------------------------------------------
// Prompt handling
// ---------------------------------------------------------------------------

/// Handle a `session/prompt` request: run the agent loop and stream events.
async fn handle_prompt(
    req: schema::PromptRequest,
    responder: Responder<schema::PromptResponse>,
    cx: ConnectionTo<UntypedRole>,
    state: &AcpState,
) -> Result<(), agent_client_protocol::Error> {
    let prompt_text = extract_prompt_text(&req.prompt);
    let session_id = &req.session_id;

    tracing::info!(
        "prompt: session={session_id:?}, text_len={}",
        prompt_text.len()
    );

    // Build components for this prompt turn.
    let session_service = Arc::new(crate::agent::session::InMemorySession::new());
    let profile = state
        .config
        .resolve_default_profile()
        .map_err(|e| agent_client_protocol::Error::internal_error().data(e.to_string()))?;

    let agent_loop = AgentLoop::new(
        &state.tool_registry,
        profile,
        session_service,
        "xylitol".to_owned(),
        Some(&state.config.hooks),
    )
    .await
    .map_err(|e| {
        agent_client_protocol::Error::internal_error().data(format!("build agent loop: {e}"))
    })?;

    let repeat_cfg = state
        .config
        .repeat_detection
        .enabled
        .then(|| DetectionConfig::from(&state.config.repeat_detection));

    let mut stream = agent_loop
        .run(&prompt_text, &session_id.to_string(), repeat_cfg)
        .await
        .map_err(|e| {
            agent_client_protocol::Error::internal_error().data(format!("run agent loop: {e}"))
        })?;

    // Stream agent events as ACP session/update notifications.
    while let Some(event) = stream.next().await {
        send_event_notification(&event, session_id, &cx)?;
    }

    // Respond with end-turn.
    tracing::info!("prompt complete: session={session_id:?}");
    responder.respond(schema::PromptResponse::new(schema::StopReason::EndTurn))
}

// ---------------------------------------------------------------------------
// AgentEvent → ACP notification conversion
// ---------------------------------------------------------------------------

/// Convert an [`AgentEvent`] to an ACP session/update notification and send it.
fn send_event_notification(
    event: &AgentEvent,
    session_id: &schema::SessionId,
    cx: &ConnectionTo<UntypedRole>,
) -> Result<(), agent_client_protocol::Error> {
    use schema::{
        AgentNotification, ContentBlock, ContentChunk, SessionNotification, SessionUpdate,
        TextContent, ToolCall, ToolCallStatus, ToolCallUpdate, ToolCallUpdateFields,
    };

    match event {
        AgentEvent::TextDelta(text) => {
            let notif = SessionNotification::new(
                session_id.clone(),
                SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(
                    TextContent::new(text.clone()),
                ))),
            );
            cx.send_notification(AgentNotification::SessionNotification(notif))
        }

        AgentEvent::ThinkingDelta(_thinking) => {
            // ACP channel does not have a dedicated "reasoning" block in this MVP.
            // Skip to avoid leaking chain-of-thought in IDE integrations.
            Ok(())
        }

        AgentEvent::ToolCallStart { id, name, args } => {
            let update = SessionUpdate::ToolCall(
                ToolCall::new(id.clone(), name.clone())
                    .kind(schema::ToolKind::Other)
                    .status(ToolCallStatus::InProgress)
                    .raw_input(args.clone()),
            );
            let notif = SessionNotification::new(session_id.clone(), update);
            cx.send_notification(AgentNotification::SessionNotification(notif))
        }

        AgentEvent::ToolCallEnd { id, result } => {
            let update = SessionUpdate::ToolCallUpdate(ToolCallUpdate::new(
                id.clone(),
                ToolCallUpdateFields::new()
                    .status(ToolCallStatus::Completed)
                    .raw_output(result.clone()),
            ));
            let notif = SessionNotification::new(session_id.clone(), update);
            cx.send_notification(AgentNotification::SessionNotification(notif))
        }

        AgentEvent::StepComplete { .. } => {
            // StepComplete is mapped to a TurnEnd, but ACP uses the
            // prompt response instead. Nothing to send here.
            Ok(())
        }

        AgentEvent::Error(err) => {
            let notif = SessionNotification::new(
                session_id.clone(),
                SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(
                    TextContent::new(format!("Error: {err}")),
                ))),
            );
            cx.send_notification(AgentNotification::SessionNotification(notif))
        }

        AgentEvent::RepeatDetected {
            consecutive_hits,
            window_repeat_ratio,
        } => {
            let msg = format!(
                "[Repeat detected: hits={consecutive_hits}, ratio={window_repeat_ratio:.2}]"
            );
            let notif = SessionNotification::new(
                session_id.clone(),
                SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(
                    TextContent::new(msg),
                ))),
            );
            cx.send_notification(AgentNotification::SessionNotification(notif))
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract text from a list of [`schema::ContentBlock`]s.
fn extract_prompt_text(blocks: &[schema::ContentBlock]) -> String {
    blocks
        .iter()
        .filter_map(|block| match block {
            schema::ContentBlock::Text(t) => Some(t.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Build the tool registry with builtins, optional MCP tools, and security wrapping.
async fn build_tool_registry(config: &AppConfig) -> Result<ToolRegistry, anyhow::Error> {
    let mut tools = ToolRegistry::builtins();

    #[cfg(feature = "infra-skills")]
    {
        let manager = std::sync::Arc::new(crate::infra::skills::McpClientManager::new());
        manager
            .connect(config)
            .await
            .map_err(|e| anyhow::anyhow!("MCP connect: {e}"))?;
        let mcp_tools = manager.list_all_tools().await;
        for (server_id, tool_name, description, schema_json) in mcp_tools {
            tools.register(std::sync::Arc::new(
                crate::infra::skills::McpToolAdapter::new(
                    server_id,
                    tool_name,
                    description,
                    Some(schema_json),
                    manager.clone(),
                ),
            ));
        }
    }

    if config.security.enabled {
        let engine = crate::infra::security::SecurityEngine::new(&config.security);
        tools.wrap_with_security(engine);
    }

    Ok(tools)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ── extract_prompt_text ────────────────────────────────────────────

    #[test]
    fn test_extract_prompt_text_empty() {
        assert_eq!(extract_prompt_text(&[]), "");
    }

    #[test]
    fn test_extract_prompt_text_single() {
        let blocks = [schema::ContentBlock::Text(schema::TextContent::new(
            "hello",
        ))];
        assert_eq!(extract_prompt_text(&blocks), "hello");
    }

    #[test]
    fn test_extract_prompt_text_multiple() {
        let blocks = [
            schema::ContentBlock::Text(schema::TextContent::new("line one")),
            schema::ContentBlock::Text(schema::TextContent::new("line two")),
        ];
        assert_eq!(extract_prompt_text(&blocks), "line one\nline two");
    }

    #[test]
    fn test_extract_prompt_text_skips_non_text() {
        use schema::ImageContent;
        use schema::{ContentBlock, TextContent};

        let blocks = [
            ContentBlock::Text(TextContent::new("hello")),
            ContentBlock::Image(ImageContent::new("data:image/png;base64,...", "image/png")),
            ContentBlock::Text(TextContent::new("world")),
        ];
        assert_eq!(extract_prompt_text(&blocks), "hello\nworld");
    }

    // ── Schema construction ────────────────────────────────────────────

    /// Verify that the ACP schema types used in our handlers can be
    /// constructed without panicking.
    #[test]
    fn test_initialize_request_roundtrip() {
        let req = schema::InitializeRequest::new(schema::ProtocolVersion::V1);
        let json = serde_json::to_value(&req).unwrap();
        // ProtocolVersion::V1 serializes as a number (1), not a string
        assert_eq!(json["protocolVersion"], 1);
    }

    #[test]
    fn test_new_session_request() {
        let req = schema::NewSessionRequest::new(std::path::Path::new("/tmp"));
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["cwd"], "/tmp");
    }

    #[test]
    fn test_agent_notification_wrapping() {
        use schema::{ContentBlock, ContentChunk, SessionNotification, SessionUpdate, TextContent};

        let session_id = schema::SessionId::new("test-session");
        let notif = SessionNotification::new(
            session_id,
            SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(
                TextContent::new("hello"),
            ))),
        );
        let json = serde_json::to_value(&notif).unwrap();
        assert_eq!(json["sessionId"], "test-session");
        assert_eq!(json["update"]["sessionUpdate"], "agent_message_chunk");
    }
}
