//! REST API — HTTP control surface for the server.
//!
//! Routes under `/api/v1`. All responses use the [`protocol::Envelope`] format.
//! The actual agent integration is wired in `runtime.rs`; this module defines
//! the route handlers and their shape.

use std::sync::Arc;

use axum::{
    Router,
    extract::{
        Path, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::{IntoResponse, Json},
    routing::{delete, get, post},
};

use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::Mutex;

use crate::app::core::driver::{Driver, InProcessDriver};
use crate::app::server::ws::{ClientFrame, EventJournal, ReverseRpcGateway, ServerFrame};
use crate::domain::lifecycle::XyEvent;
use crate::protocol::{Envelope, ErrorCode};

// ── Shared application state ───────────────────────────────────────

/// Shared state available to all route handlers.
///
/// Holds [`InProcessDriver`] (same seam as Print), not a bare `ReActAgent`.
#[derive(Clone)]
pub struct AppState {
    pub driver: Arc<Mutex<InProcessDriver>>,
    pub journal: Arc<Mutex<EventJournal>>,
    pub gateway: Arc<ReverseRpcGateway>,
}

// ── Route handlers ─────────────────────────────────────────────────

/// Health-check endpoint.
async fn healthz(State(_state): State<Arc<AppState>>) -> Json<Envelope<Value>> {
    Json(Envelope::ok(serde_json::json!({"status": "ok"})))
}

/// Request body for the run-prompt endpoint.
#[derive(Deserialize)]
pub struct RunPromptBody {
    pub prompt: Option<String>,
}

/// Submit a prompt to a session.
///
/// Starts the agent loop in a background task and writes events to the
/// journal. Returns immediately with the session_id. The client polls
/// GET /events for results or subscribes via WebSocket.
async fn run_prompt(
    Path(_session_id): Path<String>,
    State(state): State<Arc<AppState>>,
    axum::extract::Json(body): axum::extract::Json<RunPromptBody>,
) -> Json<Envelope<Value>> {
    let prompt = match body.prompt {
        Some(p) => p,
        None => {
            return Json(Envelope::error(
                ErrorCode::BadRequest,
                "missing required field: prompt",
            ));
        }
    };

    let driver = state.driver.clone();
    let journal = state.journal.clone();

    // Spawn a background task that runs via Driver and records events.
    // Lock is held only to obtain the stream (same pattern as before with agent).
    tokio::spawn(async move {
        let stream = {
            let mut driver = driver.lock().await;
            driver.run(&prompt).await
        };
        let mut stream = stream;
        while let Some(event) = stream.next().await {
            if let Some(pe) = event.to_wire_event() {
                let mut j = journal.lock().await;
                j.append(pe);
            }
            // If the stream ended, break.
            if matches!(event, XyEvent::AgentEnd { .. }) {
                break;
            }
        }
    });

    Json(Envelope::ok(serde_json::json!({"session_id": _session_id})))
}

/// Cancel the active turn in a session.
async fn cancel_session(
    Path(_session_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Json<Envelope<Value>> {
    state.driver.lock().await.abort();
    Json(Envelope::ok(serde_json::json!({"cancelled": true})))
}

/// Query parameters for the model-switch endpoint.
#[derive(Deserialize)]
pub struct SwitchModelParams {
    pub model_id: String,
}

/// Switch the model for a session.
async fn switch_model(
    Path(_session_id): Path<String>,
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<SwitchModelParams>,
) -> Json<Envelope<Value>> {
    let mut driver = state.driver.lock().await;
    match driver.select_model(&params.model_id) {
        Ok(model) => Json(Envelope::ok(serde_json::json!({
            "model": model.id,
            "display_name": model.display_name,
        }))),
        Err(msg) => Json(Envelope::error(ErrorCode::BadRequest, msg)),
    }
}

/// Query parameters for the events endpoint.
#[derive(Deserialize)]
pub struct EventsParams {
    pub seq: Option<u64>,
}

/// Poll events from a session (long-poll fallback).
async fn get_events(
    Path(_session_id): Path<String>,
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<EventsParams>,
) -> Json<Envelope<Value>> {
    let from_seq = params.seq.unwrap_or(0);
    let journal = state.journal.lock().await;
    let events = match journal.replay_from(from_seq) {
        Some(evts) => {
            let max = journal.max_seq();
            let items: Vec<Value> = evts
                .iter()
                .map(|(seq, event)| {
                    serde_json::json!({
                        "seq": seq,
                        "event": serde_json::to_value(event).unwrap_or_default(),
                    })
                })
                .collect();
            serde_json::json!({"events": items, "max_seq": max})
        }
        None => {
            serde_json::json!({"error": "journal truncated, resync required", "resync": true})
        }
    };
    Json(Envelope::ok(events))
}

// ── WebSocket handler ─────────────────────────────────────────────

/// Handle WebSocket upgrade for session event streaming.
async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(session_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, session_id, state))
}

async fn handle_ws(socket: WebSocket, session_id: String, state: Arc<AppState>) {
    use futures::StreamExt;
    let (mut sender, mut receiver) = socket.split();

    // Wait for Subscribe frame.
    // Initial 0 is always overwritten by the Subscribe frame's `seq` before
    // the first `replay_from` read; retained for the borrow checker.
    #[allow(unused_assignments)]
    let mut last_seq = 0u64;

    while let Some(msg) = receiver.next().await {
        let msg = match msg {
            Ok(Message::Text(text)) => text,
            Ok(Message::Close(_)) | Err(_) => break,
            _ => continue,
        };

        let frame: ClientFrame = match serde_json::from_str(&msg) {
            Ok(f) => f,
            Err(_) => continue,
        };

        match frame {
            ClientFrame::Subscribe { last_seq: seq, .. } => {
                last_seq = seq;

                // Send ServerHello + Ack
                let hello = serde_json::to_string(&ServerFrame::ServerHello {
                    version: "1.0".into(),
                })
                .unwrap();
                let ack = serde_json::to_string(&ServerFrame::Ack {
                    seq: 0,
                    request_id: None,
                })
                .unwrap();

                if sender.send(Message::Text(hello.into())).await.is_err() {
                    break;
                }
                if sender.send(Message::Text(ack.into())).await.is_err() {
                    break;
                }

                // Replay events from last_seq
                {
                    let journal = state.journal.lock().await;
                    if let Some(events) = journal.replay_from(last_seq) {
                        for (seq, event) in events {
                            let frame = serde_json::to_string(&ServerFrame::Event {
                                session_id: session_id.clone(),
                                seq,
                                event: event.clone(),
                            })
                            .unwrap();
                            if sender.send(Message::Text(frame.into())).await.is_err() {
                                break;
                            }
                        }
                    } else {
                        let resync = serde_json::to_string(&ServerFrame::ResyncRequired {
                            session_id: session_id.clone(),
                        })
                        .unwrap();
                        let _ = sender.send(Message::Text(resync.into())).await;
                    }
                }

                // Stream new events by polling the journal
                loop {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    let journal = state.journal.lock().await;
                    if let Some(events) = journal.replay_from(last_seq) {
                        for (seq, event) in &events {
                            let frame = serde_json::to_string(&ServerFrame::Event {
                                session_id: session_id.clone(),
                                seq: *seq,
                                event: event.clone(),
                            })
                            .unwrap();
                            if sender.send(Message::Text(frame.into())).await.is_err() {
                                return;
                            }
                        }
                        if let Some((max_seq, _)) = events.last() {
                            last_seq = *max_seq;
                        }
                    }
                }
            }
            ClientFrame::ApproveTool { call_id, approved } => {
                state.gateway.handle_approve(&call_id, approved);
            }
            ClientFrame::AnswerQuestion { call_id, answer } => {
                state.gateway.handle_answer(&call_id, answer);
            }
            ClientFrame::Ping => {
                // No-op, keep-alive
            }
        }
    }
}

// ── Router construction ────────────────────────────────────────────

/// Build the `/api/v1` router with the given shared state.
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/v1/healthz", get(healthz))
        .route("/api/v1/session/{id}/run", post(run_prompt))
        .route("/api/v1/session/{id}", delete(cancel_session))
        .route("/api/v1/session/{id}/model", post(switch_model))
        .route("/api/v1/session/{id}/events", get(get_events))
        .route("/api/v1/session/{id}/ws", get(ws_handler))
        .with_state(state)
}
