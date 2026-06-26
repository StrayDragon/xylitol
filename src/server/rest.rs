//! REST API — HTTP control surface for the server.
//!
//! Routes under `/api/v1`. All responses use the [`protocol::Envelope`] format.
//! The actual agent integration is wired in `runtime.rs`; this module defines
//! the route handlers and their shape.

use std::sync::Arc;

use axum::{
    Router,
    extract::{Path, State},
    response::Json,
    routing::{delete, get, post},
};

use serde::Deserialize;
use serde_json::Value;

use crate::protocol::{Envelope, ErrorCode};

// ── Shared application state ───────────────────────────────────────

/// Shared state available to all route handlers.
#[derive(Clone)]
pub struct AppState {
    // Placeholder for agent runtime — populated in runtime.rs.
    // pub agent: Arc<Mutex<Agent>>,
    // pub store: Arc<dyn SessionStore>,
    // pub sink: Arc<dyn EventSink>,
}

// ── Route handlers ─────────────────────────────────────────────────

/// Health-check endpoint.
async fn healthz() -> Json<Envelope<Value>> {
    Json(Envelope::ok(serde_json::json!({"status": "ok"})))
}

/// Submit a prompt to a session.
async fn run_prompt(
    Path(_session_id): Path<String>,
    State(_state): State<Arc<AppState>>,
) -> Json<Envelope<Value>> {
    Json(Envelope::error(
        ErrorCode::InternalError,
        "not yet implemented — runtime assembly in progress",
    ))
}

/// Cancel the active turn in a session.
async fn cancel_session(
    Path(_session_id): Path<String>,
    State(_state): State<Arc<AppState>>,
) -> Json<Envelope<Value>> {
    Json(Envelope::error(
        ErrorCode::InternalError,
        "not yet implemented — runtime assembly in progress",
    ))
}

/// Query parameters for the model-switch endpoint.
#[derive(Deserialize)]
pub struct SwitchModelParams {
    pub model_id: String,
}

/// Switch the model for a session.
async fn switch_model(
    Path(_session_id): Path<String>,
    State(_state): State<Arc<AppState>>,
    axum::extract::Query(_params): axum::extract::Query<SwitchModelParams>,
) -> Json<Envelope<Value>> {
    Json(Envelope::error(
        ErrorCode::InternalError,
        "not yet implemented — runtime assembly in progress",
    ))
}

/// Query parameters for the events endpoint.
#[derive(Deserialize)]
pub struct EventsParams {
    pub seq: Option<u64>,
}

/// Poll events from a session (long-poll fallback).
async fn get_events(
    Path(_session_id): Path<String>,
    State(_state): State<Arc<AppState>>,
    axum::extract::Query(_params): axum::extract::Query<EventsParams>,
) -> Json<Envelope<Value>> {
    Json(Envelope::error(
        ErrorCode::InternalError,
        "not yet implemented — runtime assembly in progress",
    ))
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
        .with_state(state)
}
