//! REST API — HTTP control surface for the server.
//!
//! Routes under `/api/v1`. All responses use the [`crate::protocol::Envelope`] format.
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

use crate::app::core::dispatch::{DispatchOutcome, XyDriverError, dispatch};
use crate::app::core::driver::{XyDriver, XyInProcessDriver};
use crate::app::server::ws::{ClientFrame, EventJournal, ReverseRpcGateway, ServerFrame};
use crate::protocol::lifecycle::XyEvent;
use crate::protocol::session::SessionTreeKind;
use crate::protocol::{Command, Envelope, ErrorCode};

// ── Shared application state ───────────────────────────────────────

/// Shared state available to all route handlers.
///
/// Holds [`XyInProcessDriver`] (same seam as Print), not a bare `AgentRuntime`.
#[derive(Clone)]
pub struct AppState {
    pub driver: Arc<Mutex<XyInProcessDriver>>,
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

    // Spawn a background task that runs via XyDriver and records events.
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
    map_dispatch(
        run_dispatch(
            &state,
            Command::SetModel {
                id: None,
                provider: String::new(),
                model_id: params.model_id,
            },
        )
        .await,
        |o| match o {
            DispatchOutcome::Model(model) => Some(serde_json::json!({
                "model": model.id,
                "display_name": model.display_name,
            })),
            _ => None,
        },
        dispatch_err,
    )
}

async fn run_dispatch(state: &AppState, cmd: Command) -> Result<DispatchOutcome, XyDriverError> {
    let mut driver = state.driver.lock().await;
    dispatch(&mut *driver, cmd).await
}

fn unexpected_outcome() -> Json<Envelope<Value>> {
    Json(Envelope::error(
        ErrorCode::InternalError,
        "unexpected dispatch outcome",
    ))
}

fn dispatch_err(e: XyDriverError) -> Json<Envelope<Value>> {
    Json(Envelope::error(ErrorCode::BadRequest, e.to_string()))
}

fn dispatch_err_as(e: XyDriverError, code: ErrorCode) -> Json<Envelope<Value>> {
    Json(Envelope::error(code, e.to_string()))
}

/// Map a dispatch result: `map` returns `Some(data)` on the expected variant.
fn map_dispatch(
    result: Result<DispatchOutcome, XyDriverError>,
    map: impl FnOnce(DispatchOutcome) -> Option<Value>,
    on_err: impl FnOnce(XyDriverError) -> Json<Envelope<Value>>,
) -> Json<Envelope<Value>> {
    match result {
        Ok(outcome) => match map(outcome) {
            Some(data) => Json(Envelope::ok(data)),
            None => unexpected_outcome(),
        },
        Err(e) => on_err(e),
    }
}

fn queue_json(steer_count: usize, follow_up_count: usize) -> Value {
    serde_json::json!({
        "steer_count": steer_count,
        "follow_up_count": follow_up_count,
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
    map_dispatch(
        run_dispatch(
            &state,
            Command::Steer {
                id: None,
                message: body.message,
            },
        )
        .await,
        |o| match o {
            DispatchOutcome::QueueStats {
                steer_count,
                follow_up_count,
            } => Some(queue_json(steer_count, follow_up_count)),
            _ => None,
        },
        dispatch_err,
    )
}

async fn follow_up(
    Path(_session_id): Path<String>,
    State(state): State<Arc<AppState>>,
    axum::extract::Json(body): axum::extract::Json<MessageBody>,
) -> Json<Envelope<Value>> {
    map_dispatch(
        run_dispatch(
            &state,
            Command::FollowUp {
                id: None,
                message: body.message,
            },
        )
        .await,
        |o| match o {
            DispatchOutcome::QueueStats {
                steer_count,
                follow_up_count,
            } => Some(queue_json(steer_count, follow_up_count)),
            _ => None,
        },
        dispatch_err,
    )
}

async fn clear_queue(
    Path(_session_id): Path<String>,
    State(state): State<Arc<AppState>>,
    axum::extract::Json(body): axum::extract::Json<ClearQueueBody>,
) -> Json<Envelope<Value>> {
    map_dispatch(
        run_dispatch(
            &state,
            Command::ClearQueue {
                id: None,
                clear_steer: body.clear_steer,
                clear_follow_up: body.clear_follow_up,
            },
        )
        .await,
        |o| match o {
            DispatchOutcome::QueueStats {
                steer_count,
                follow_up_count,
            } => Some(queue_json(steer_count, follow_up_count)),
            _ => None,
        },
        dispatch_err,
    )
}

async fn get_queue(
    Path(_session_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Json<Envelope<Value>> {
    // No GetQueue Command — thin XyDriver read (design c550).
    let driver = state.driver.lock().await;
    let s = driver.queue_stats();
    Json(Envelope::ok(queue_json(s.steer_count, s.follow_up_count)))
}

async fn get_state(
    Path(_session_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Json<Envelope<Value>> {
    map_dispatch(
        run_dispatch(&state, Command::GetState { id: None }).await,
        |o| match o {
            DispatchOutcome::State(st) => Some(serde_json::json!({
                "session_id": st.session_id,
                "model": st.model.as_ref().map(model_data),
                "thinking_level": st.thinking_level,
            })),
            _ => None,
        },
        dispatch_err,
    )
}

async fn list_models(
    Path(_session_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Json<Envelope<Value>> {
    map_dispatch(
        run_dispatch(&state, Command::GetAvailableModels { id: None }).await,
        |o| match o {
            DispatchOutcome::Models(models) => {
                let models: Vec<Value> = models.iter().map(model_data).collect();
                Some(serde_json::json!({ "models": models }))
            }
            _ => None,
        },
        dispatch_err,
    )
}

async fn cycle_model(
    Path(_session_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Json<Envelope<Value>> {
    map_dispatch(
        run_dispatch(&state, Command::CycleModel { id: None }).await,
        |o| match o {
            DispatchOutcome::Model(model) => Some(model_data(&model)),
            _ => None,
        },
        dispatch_err,
    )
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
    map_dispatch(
        run_dispatch(
            &state,
            Command::SetThinkingLevel {
                id: None,
                level: body.level,
            },
        )
        .await,
        |o| match o {
            DispatchOutcome::ThinkingLevel(level) => {
                Some(serde_json::json!({ "thinking_level": level }))
            }
            _ => None,
        },
        dispatch_err,
    )
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
    map_dispatch(
        run_dispatch(
            &state,
            Command::Bash {
                id: None,
                command: body.command,
                exclude_from_context: body.exclude_from_context,
            },
        )
        .await,
        |o| match o {
            DispatchOutcome::Bash(r) => Some(serde_json::json!({
                "output": r.output,
                "exit_code": r.exit_code,
                "cancelled": r.cancelled,
                "truncated": r.truncated,
            })),
            _ => None,
        },
        |e| dispatch_err_as(e, ErrorCode::InternalError),
    )
}

async fn compact(
    Path(_session_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Json<Envelope<Value>> {
    map_dispatch(
        run_dispatch(&state, Command::Compact { id: None }).await,
        |o| match o {
            DispatchOutcome::Compacted(did) => Some(serde_json::json!({ "compacted": did })),
            _ => None,
        },
        |e| dispatch_err_as(e, ErrorCode::InternalError),
    )
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
    map_dispatch(
        run_dispatch(
            &state,
            Command::ExportHtml {
                id: None,
                output_path: Some(body.path),
            },
        )
        .await,
        |o| match o {
            DispatchOutcome::ExportedPath(p) => Some(serde_json::json!({ "path": p })),
            _ => None,
        },
        |e| dispatch_err_as(e, ErrorCode::InternalError),
    )
}

async fn export_jsonl(
    Path(_session_id): Path<String>,
    State(state): State<Arc<AppState>>,
    axum::extract::Json(body): axum::extract::Json<PathBody>,
) -> Json<Envelope<Value>> {
    map_dispatch(
        run_dispatch(
            &state,
            Command::ExportJsonl {
                id: None,
                output_path: Some(body.path),
            },
        )
        .await,
        |o| match o {
            DispatchOutcome::ExportedPath(p) => Some(serde_json::json!({ "path": p })),
            _ => None,
        },
        |e| dispatch_err_as(e, ErrorCode::InternalError),
    )
}

async fn import_jsonl(
    Path(_session_id): Path<String>,
    State(state): State<Arc<AppState>>,
    axum::extract::Json(body): axum::extract::Json<PathBody>,
) -> Json<Envelope<Value>> {
    map_dispatch(
        run_dispatch(
            &state,
            Command::ImportJsonl {
                id: None,
                input_path: body.path,
            },
        )
        .await,
        |o| match o {
            DispatchOutcome::NewSession(id) => Some(serde_json::json!({ "session_id": id })),
            _ => None,
        },
        |e| dispatch_err_as(e, ErrorCode::InternalError),
    )
}

#[derive(Deserialize)]
struct ForkBody {
    entry_id: String,
    #[serde(default)]
    position: Option<String>,
}

async fn fork_session(
    Path(_session_id): Path<String>,
    State(state): State<Arc<AppState>>,
    axum::extract::Json(body): axum::extract::Json<ForkBody>,
) -> Json<Envelope<Value>> {
    map_dispatch(
        run_dispatch(
            &state,
            Command::Fork {
                id: None,
                entry_id: body.entry_id,
                position: body.position,
            },
        )
        .await,
        |o| match o {
            DispatchOutcome::NewSession(id) => Some(serde_json::json!({ "session_id": id })),
            _ => None,
        },
        |e| dispatch_err_as(e, ErrorCode::InternalError),
    )
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
    map_dispatch(
        run_dispatch(
            &state,
            Command::SwitchSession {
                id: None,
                session_path: body.session_id,
            },
        )
        .await,
        |o| match o {
            DispatchOutcome::SwitchedSession(id) => Some(serde_json::json!({ "session_id": id })),
            _ => None,
        },
        |e| dispatch_err_as(e, ErrorCode::NotFound),
    )
}

async fn get_messages(
    Path(_session_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Json<Envelope<Value>> {
    map_dispatch(
        run_dispatch(&state, Command::GetMessages { id: None }).await,
        |o| match o {
            DispatchOutcome::Messages { entries, .. } => serde_json::to_value(entries)
                .ok()
                .map(|v| serde_json::json!({ "entries": v })),
            _ => None,
        },
        |e| dispatch_err_as(e, ErrorCode::InternalError),
    )
}

async fn get_session_stats(
    Path(_session_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Json<Envelope<Value>> {
    map_dispatch(
        run_dispatch(&state, Command::GetSessionStats { id: None }).await,
        |o| match o {
            DispatchOutcome::SessionStats(v) => Some(v),
            _ => None,
        },
        |e| dispatch_err_as(e, ErrorCode::InternalError),
    )
}

async fn get_commands(
    Path(_session_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Json<Envelope<Value>> {
    map_dispatch(
        run_dispatch(&state, Command::GetCommands { id: None }).await,
        |o| match o {
            DispatchOutcome::Commands(cmds) => {
                let cmds: Vec<Value> = cmds
                    .into_iter()
                    .map(|c| serde_json::json!({ "name": c.name, "description": c.description }))
                    .collect();
                Some(serde_json::json!({ "commands": cmds }))
            }
            _ => None,
        },
        dispatch_err,
    )
}

/// XyDriver-only: read MessageHistory session tree (not routed through `protocol::Command`).
async fn get_message_history_tree(
    Path(_session_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Json<Envelope<Value>> {
    let driver = state.driver.lock().await;
    match driver.session_tree(SessionTreeKind::MessageHistory).await {
        Ok(tree) => match serde_json::to_value(tree) {
            Ok(v) => Json(Envelope::ok(serde_json::json!({ "tree": v }))),
            Err(e) => Json(Envelope::error(ErrorCode::InternalError, e.to_string())),
        },
        Err(e) => Json(Envelope::error(ErrorCode::BadRequest, e.to_string())),
    }
}

#[derive(Deserialize)]
struct TravelTreeBody {
    entry_id: String,
}

/// XyDriver-only: travel MessageHistory tree (not routed through `protocol::Command`).
async fn travel_message_history_tree(
    Path(_session_id): Path<String>,
    State(state): State<Arc<AppState>>,
    axum::extract::Json(body): axum::extract::Json<TravelTreeBody>,
) -> Json<Envelope<Value>> {
    let driver = state.driver.lock().await;
    match driver
        .travel_session_tree(SessionTreeKind::MessageHistory, &body.entry_id)
        .await
    {
        Ok(travel) => match serde_json::to_value(travel) {
            Ok(v) => Json(Envelope::ok(v)),
            Err(e) => Json(Envelope::error(ErrorCode::InternalError, e.to_string())),
        },
        Err(e) => Json(Envelope::error(ErrorCode::BadRequest, e.to_string())),
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
        .route(
            "/api/v1/session/{id}/trees/message-history",
            get(get_message_history_tree),
        )
        .route(
            "/api/v1/session/{id}/trees/message-history/travel",
            post(travel_message_history_tree),
        )
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
    use crate::protocol::ports::XySessionStore;

    fn test_state() -> Arc<AppState> {
        let agent = build_agent(BuildAgentOptions::default()).expect("build");
        let store: Arc<dyn XySessionStore> = Arc::new(crate::infra::session::SessionManager::new(
            tempfile::tempdir().unwrap().path().join("sessions"),
        ));
        let driver = XyInProcessDriver::new(agent, store);
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
        let outcome = run_dispatch(
            &state,
            Command::Steer {
                id: None,
                message: "inject".into(),
            },
        )
        .await
        .expect("dispatch steer");
        match outcome {
            DispatchOutcome::QueueStats {
                steer_count,
                follow_up_count,
            } => {
                assert_eq!(steer_count, 1);
                assert_eq!(follow_up_count, 0);
                let data = queue_json(steer_count, follow_up_count);
                assert_eq!(data["steer_count"], 1);
            }
            other => panic!("unexpected outcome: {other:?}"),
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
