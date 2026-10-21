//! Lifecycle waterfall events. Per-token deltas stay on `XyDriver::run`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::rt::Event;

/// `tool/pre-execute`: `Some(reason)` rejects; `None` allows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPreExecute {
    pub name: String,
    pub args: Value,
}

impl Event for ToolPreExecute {
    const NAME: &'static str = "tool/pre-execute";
    type Value = Option<String>;
}

/// `tool/post-execute`: waterfall value is the (possibly replaced) output text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPostExecute {
    pub name: String,
    pub ok: bool,
    pub output: String,
}

impl Event for ToolPostExecute {
    const NAME: &'static str = "tool/post-execute";
    type Value = String;
}

/// `agent/pre-step`: `Err(reason)` fails the turn; `Ok(())` continues.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPreStep {
    pub step: usize,
}

impl Event for AgentPreStep {
    const NAME: &'static str = "agent/pre-step";
    type Value = Result<(), String>;
}

/// `http/before-request`: plugins may rewrite the JSON body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpBeforeRequest {
    pub model: String,
    pub body: Value,
}

impl Event for HttpBeforeRequest {
    const NAME: &'static str = "http/before-request";
    type Value = Value;
}

/// `trust/pre-load`: `Some(reason)` blocks loading project-local resources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustPreLoad {
    pub cwd: String,
}

impl Event for TrustPreLoad {
    const NAME: &'static str = "trust/pre-load";
    type Value = Option<String>;
}
