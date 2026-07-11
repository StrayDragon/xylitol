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

fn queue_data(driver: &impl Driver) -> Value {
    let s = driver.queue_stats();
    serde_json::json!({
        "steer_count": s.steer_count,
        "follow_up_count": s.follow_up_count,
    })
}

fn model_data(m: &crate::app::core::driver::ModelInfo) -> Value {
    serde_json::json!({
        "id": m.id,
        "display_name": m.display_name,
        "thinking": m.thinking,
        "context_window": m.context_window,
    })
}

#[derive(Deserialize)]
struct MessageBody {
    message: String,
}

#[derive(Deserialize)]
struct ClearQueueBody {
    #[serde(default = "default_true")]
    clear_steer: bool,
    #[serde(default = "default_true")]
    clear_follow_up: bool,
}

fn default_true() -> bool {
    true
}

async fn steer(
    Path(_session_id): Path<String>,
    State(state): State<Arc<AppState>>,
    axum::extract::Json(body): axum::extract::Json<MessageBody>,
) -> Json<Envelope<Value>> {
    let mut driver = state.driver.lock().await;
    match driver.steer(&body.message) {
        Ok(()) => Json(Envelope::ok(queue_data(&*driver))),
        Err(msg) => Json(Envelope::error(ErrorCode::BadRequest, msg)),
    }
}

async fn follow_up(
    Path(_session_id): Path<String>,
    State(state): State<Arc<AppState>>,
    axum::extract::Json(body): axum::extract::Json<MessageBody>,
) -> Json<Envelope<Value>> {
    let mut driver = state.driver.lock().await;
    match driver.follow_up(&body.message) {
        Ok(()) => Json(Envelope::ok(queue_data(&*driver))),
        Err(msg) => Json(Envelope::error(ErrorCode::BadRequest, msg)),
    }
}

async fn clear_queue(
    Path(_session_id): Path<String>,
    State(state): State<Arc<AppState>>,
    axum::extract::Json(body): axum::extract::Json<ClearQueueBody>,
) -> Json<Envelope<Value>> {
    let mut driver = state.driver.lock().await;
    match driver.clear_queue(body.clear_steer, body.clear_follow_up) {
        Ok(()) => Json(Envelope::ok(queue_data(&*driver))),
        Err(msg) => Json(Envelope::error(ErrorCode::BadRequest, msg)),
    }
}

async fn get_queue(
    Path(_session_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Json<Envelope<Value>> {
    let driver = state.driver.lock().await;
    Json(Envelope::ok(queue_data(&*driver)))
}

async fn get_state(
    Path(_session_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Json<Envelope<Value>> {
    let driver = state.driver.lock().await;
    let st = driver.get_state();
    Json(Envelope::ok(serde_json::json!({
        "session_id": st.session_id,
        "model": st.model.as_ref().map(model_data),
        "thinking_level": st.thinking_level,
    })))
}

async fn list_models(
    Path(_session_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Json<Envelope<Value>> {
    let driver = state.driver.lock().await;
    let models: Vec<Value> = driver.available_models().iter().map(model_data).collect();
    Json(Envelope::ok(serde_json::json!({ "models": models })))
}

async fn cycle_model(
    Path(_session_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Json<Envelope<Value>> {
    let mut driver = state.driver.lock().await;
    match driver.cycle_model() {
        Ok(model) => Json(Envelope::ok(model_data(&model))),
        Err(msg) => Json(Envelope::error(ErrorCode::BadRequest, msg)),
    }
}

#[derive(Deserialize)]
struct ThinkingBody {
    level: String,
}

async fn set_thinking(
    Path(_session_id): Path<String>,
    State(state): State<Arc<AppState>>,
    axum::extract::Json(body): axum::extract::Json<ThinkingBody>,
) -> Json<Envelope<Value>> {
    let level = match crate::app::core::dispatch::parse_thinking_level(&body.level) {
        Ok(l) => l,
        Err(e) => return Json(Envelope::error(ErrorCode::BadRequest, e.0)),
    };
    let mut driver = state.driver.lock().await;
    driver.set_thinking_level(level);
    Json(Envelope::ok(serde_json::json!({ "thinking_level": level })))
}

#[derive(Deserialize)]
struct BashBody {
    command: String,
    #[serde(default)]
    exclude_from_context: bool,
}

async fn bash(
    Path(_session_id): Path<String>,
    State(state): State<Arc<AppState>>,
    axum::extract::Json(body): axum::extract::Json<BashBody>,
) -> Json<Envelope<Value>> {
    let mut driver = state.driver.lock().await;
    match driver
        .execute_bash(&body.command, body.exclude_from_context)
        .await
    {
        Ok(r) => Json(Envelope::ok(serde_json::json!({
            "output": r.output,
            "exit_code": r.exit_code,
            "cancelled": r.cancelled,
            "truncated": r.truncated,
        }))),
        Err(msg) => Json(Envelope::error(ErrorCode::InternalError, msg)),
    }
}

async fn compact(
    Path(_session_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Json<Envelope<Value>> {
    let mut driver = state.driver.lock().await;
    match driver.compact().await {
        Ok(did) => Json(Envelope::ok(serde_json::json!({ "compacted": did }))),
        Err(msg) => Json(Envelope::error(ErrorCode::InternalError, msg)),
    }
}

#[derive(Deserialize)]
struct PathBody {
    path: String,
}

async fn export_html(
    Path(_session_id): Path<String>,
    State(state): State<Arc<AppState>>,
    axum::extract::Json(body): axum::extract::Json<PathBody>,
) -> Json<Envelope<Value>> {
    let mut driver = state.driver.lock().await;
    match driver.export_html(std::path::Path::new(&body.path)).await {
        Ok(p) => Json(Envelope::ok(serde_json::json!({ "path": p }))),
        Err(msg) => Json(Envelope::error(ErrorCode::InternalError, msg)),
    }
}

async fn export_jsonl(
    Path(_session_id): Path<String>,
    State(state): State<Arc<AppState>>,
    axum::extract::Json(body): axum::extract::Json<PathBody>,
) -> Json<Envelope<Value>> {
    let mut driver = state.driver.lock().await;
    match driver.export_jsonl(std::path::Path::new(&body.path)).await {
        Ok(p) => Json(Envelope::ok(serde_json::json!({ "path": p }))),
        Err(msg) => Json(Envelope::error(ErrorCode::InternalError, msg)),
    }
}

async fn import_jsonl(
    Path(_session_id): Path<String>,
    State(state): State<Arc<AppState>>,
    axum::extract::Json(body): axum::extract::Json<PathBody>,
) -> Json<Envelope<Value>> {
    let mut driver = state.driver.lock().await;
    match driver.import_jsonl(std::path::Path::new(&body.path)).await {
        Ok(id) => Json(Envelope::ok(serde_json::json!({ "session_id": id }))),
        Err(msg) => Json(Envelope::error(ErrorCode::InternalError, msg)),
    }
}

#[derive(Deserialize)]
struct ForkBody {
    entry_id: String,
}

async fn fork_session(
    Path(_session_id): Path<String>,
    State(state): State<Arc<AppState>>,
    axum::extract::Json(body): axum::extract::Json<ForkBody>,
) -> Json<Envelope<Value>> {
    let mut driver = state.driver.lock().await;
    match driver.fork_session(&body.entry_id).await {
        Ok(id) => Json(Envelope::ok(serde_json::json!({ "session_id": id }))),
        Err(msg) => Json(Envelope::error(ErrorCode::InternalError, msg)),
    }
}

#[derive(Deserialize)]
struct SwitchSessionBody {
    session_id: String,
}

async fn switch_session(
    Path(_session_id): Path<String>,
    State(state): State<Arc<AppState>>,
    axum::extract::Json(body): axum::extract::Json<SwitchSessionBody>,
) -> Json<Envelope<Value>> {
    let mut driver = state.driver.lock().await;
    match driver.switch_session(&body.session_id).await {
        Ok(id) => Json(Envelope::ok(serde_json::json!({ "session_id": id }))),
        Err(msg) => Json(Envelope::error(ErrorCode::NotFound, msg)),
    }
}

async fn get_messages(
    Path(_session_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Json<Envelope<Value>> {
    let driver = state.driver.lock().await;
    match driver.get_messages().await {
        Ok(entries) => match serde_json::to_value(entries) {
            Ok(v) => Json(Envelope::ok(serde_json::json!({ "entries": v }))),
            Err(e) => Json(Envelope::error(ErrorCode::InternalError, e.to_string())),
        },
        Err(msg) => Json(Envelope::error(ErrorCode::InternalError, msg)),
    }
}

async fn get_session_stats(
    Path(_session_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Json<Envelope<Value>> {
    let driver = state.driver.lock().await;
    match driver.get_session_stats().await {
        Ok(stats) => Json(Envelope::ok(serde_json::json!({
            "session_id": stats.session_id,
            "user_messages": stats.user_messages,
            "assistant_messages": stats.assistant_messages,
            "total_messages": stats.total_messages,
            "thinking_level": stats.thinking_level,
            "model": stats.model.map(|(p, m)| serde_json::json!({"provider": p, "model_id": m})),
        }))),
        Err(msg) => Json(Envelope::error(ErrorCode::InternalError, msg)),
    }
}

async fn get_commands(
    Path(_session_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Json<Envelope<Value>> {
    let driver = state.driver.lock().await;
    let cmds: Vec<Value> = driver
        .get_commands()
        .into_iter()
        .map(|c| serde_json::json!({ "name": c.name, "description": c.description }))
        .collect();
    Json(Envelope::ok(serde_json::json!({ "commands": cmds })))
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
        .route("/api/v1/session/{id}/model/cycle", post(cycle_model))
        .route("/api/v1/session/{id}/models", get(list_models))
        .route("/api/v1/session/{id}/state", get(get_state))
        .route("/api/v1/session/{id}/thinking", post(set_thinking))
        .route("/api/v1/session/{id}/steer", post(steer))
        .route("/api/v1/session/{id}/follow-up", post(follow_up))
        .route("/api/v1/session/{id}/queue", get(get_queue))
        .route("/api/v1/session/{id}/queue/clear", post(clear_queue))
        .route("/api/v1/session/{id}/bash", post(bash))
        .route("/api/v1/session/{id}/compact", post(compact))
        .route("/api/v1/session/{id}/export/html", post(export_html))
        .route("/api/v1/session/{id}/export/jsonl", post(export_jsonl))
        .route("/api/v1/session/{id}/import/jsonl", post(import_jsonl))
        .route("/api/v1/session/{id}/fork", post(fork_session))
        .route("/api/v1/session/{id}/switch", post(switch_session))
        .route("/api/v1/session/{id}/messages", get(get_messages))
        .route("/api/v1/session/{id}/stats", get(get_session_stats))
        .route("/api/v1/session/{id}/commands", get(get_commands))
        .route("/api/v1/session/{id}/events", get(get_events))
        .route("/api/v1/session/{id}/ws", get(ws_handler))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::core::composition::{BuildAgentOptions, build_agent};
    use crate::runtime_protocol::XySessionStore;

    fn test_state() -> Arc<AppState> {
        let agent = build_agent(BuildAgentOptions::default()).expect("build");
        let store: Arc<dyn XySessionStore> = Arc::new(crate::infra::session::SessionManager::new(
            tempfile::tempdir().unwrap().path().join("sessions"),
        ));
        let driver = InProcessDriver::new(agent, store);
        Arc::new(AppState {
            driver: Arc::new(Mutex::new(driver)),
            journal: Arc::new(Mutex::new(
                crate::app::server::ws::EventJournal::with_default_capacity("test"),
            )),
            gateway: Arc::new(ReverseRpcGateway::new()),
        })
    }

    #[tokio::test]
    async fn steer_queue_counts_visible_for_remote_clients() {
        let state = test_state();
        {
            let mut driver = state.driver.lock().await;
            driver.steer("inject").expect("steer");
            let stats = driver.queue_stats();
            assert_eq!(stats.steer_count, 1);
            assert_eq!(stats.follow_up_count, 0);
            // Same payload shape RemoteDriver reads from GET .../queue.
            let data = queue_data(&*driver);
            assert_eq!(data["steer_count"], 1);
            assert_eq!(data["follow_up_count"], 0);
        }
    }

    #[test]
    fn queue_update_is_appended_to_journal_via_wire() {
        let mut journal = crate::app::server::ws::EventJournal::with_default_capacity("test");
        let wire = XyEvent::QueueUpdate {
            steer_count: 3,
            follow_up_count: 1,
        }
        .to_wire_event()
        .expect("QueueUpdate on wire");
        journal.append(wire);
        let replayed = journal.replay_from(0).expect("events");
        assert_eq!(replayed.len(), 1);
        assert!(matches!(
            &replayed[0].1,
            crate::protocol::Event::QueueUpdate {
                steer_count: 3,
                follow_up_count: 1,
            }
        ));
    }
}
