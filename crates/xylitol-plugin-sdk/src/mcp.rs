use std::sync::{Arc, Mutex, OnceLock};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct McpResourceRef {
    pub server: String,
    pub uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct McpResourceContext {
    pub server: String,
    pub uri: String,
    #[serde(default)]
    pub mime: Option<String>,
    pub text: String,
    #[serde(default)]
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct McpResourceContent {
    pub server: String,
    pub uri: String,
    #[serde(default)]
    pub mime: Option<String>,
    pub text: String,
    #[serde(default)]
    pub truncated: bool,
    #[serde(default)]
    pub binary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct McpCatalogEntry {
    pub server: String,
    pub kind: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub uri: Option<String>,
    #[serde(default)]
    pub schema: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct McpElicitationRequest {
    pub server: String,
    pub message: String,
    pub schema: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum McpElicitationAction {
    Accept,
    #[default]
    Decline,
    Cancel,
}

#[derive(Debug, Clone, Default)]
pub struct McpElicitationResponse {
    pub action: McpElicitationAction,
    pub content: Option<Value>,
}

#[async_trait]
pub trait McpElicitationGateway: Send + Sync {
    async fn elicit(&self, request: McpElicitationRequest) -> McpElicitationResponse;
}

static ELICITATION_GATEWAY: OnceLock<Mutex<Option<Arc<dyn McpElicitationGateway>>>> =
    OnceLock::new();

pub fn set_mcp_elicitation_gateway(gateway: Arc<dyn McpElicitationGateway>) {
    let slot = ELICITATION_GATEWAY.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = slot.lock() {
        *guard = Some(gateway);
    }
}

pub fn mcp_elicitation_gateway() -> Option<Arc<dyn McpElicitationGateway>> {
    ELICITATION_GATEWAY
        .get()
        .and_then(|slot| slot.lock().ok())
        .and_then(|guard| guard.clone())
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct McpCatalog {
    #[serde(default)]
    pub prompts: Vec<McpCatalogEntry>,
    #[serde(default)]
    pub resources: Vec<McpCatalogEntry>,
}
