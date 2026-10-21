use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::rt::Event;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPreExecute {
    pub name: String,
    pub args: Value,
}

impl Event for ToolPreExecute {
    const NAME: &'static str = "tool/pre-execute";
    type Value = Option<String>;
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPreStep {
    pub step: usize,
}

impl Event for AgentPreStep {
    const NAME: &'static str = "agent/pre-step";
    type Value = Result<(), String>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpBeforeRequest {
    pub model: String,
    pub body: Value,
}

impl Event for HttpBeforeRequest {
    const NAME: &'static str = "http/before-request";
    type Value = Value;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustPreLoad {
    pub cwd: String,
}

impl Event for TrustPreLoad {
    const NAME: &'static str = "trust/pre-load";
    type Value = Option<String>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionPrepareReason {
    Switch,
    Resume,
    Reload,
    Generate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionPrepare {
    pub session_id: String,
    pub reason: SessionPrepareReason,
}

impl Event for SessionPrepare {
    const NAME: &'static str = "session/prepare";
    type Value = SessionPrepareResult;
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionPrepareResult {
    pub fingerprint: String,
    pub env_body: String,
}
