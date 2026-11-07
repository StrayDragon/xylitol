use std::sync::{Arc, Mutex, OnceLock};

use crate::ask::{AskModeArg, AskOptionArg, AskQuestionArg};
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpOAuthFlow {
    pub flow_id: String,
    pub server: String,
    pub auth_url: String,
    pub redirect_uri: String,
    #[serde(default)]
    pub scopes: Vec<String>,
    pub expires_at_ms: u64,
}

#[async_trait]
pub trait McpOAuthGateway: Send + Sync {
    async fn bind(&self, flow_id: &str, redirect_uri: Option<&str>) -> Result<String, String>;
    async fn wait(&self, flow_id: &str) -> Result<String, String>;
    async fn cancel(&self, flow_id: &str);
}

static OAUTH_GATEWAY: OnceLock<Mutex<Option<Arc<dyn McpOAuthGateway>>>> = OnceLock::new();

pub fn set_mcp_oauth_gateway(gateway: Arc<dyn McpOAuthGateway>) {
    let slot = OAUTH_GATEWAY.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = slot.lock() {
        *guard = Some(gateway);
    }
}

pub fn mcp_oauth_gateway() -> Option<Arc<dyn McpOAuthGateway>> {
    OAUTH_GATEWAY
        .get()
        .and_then(|slot| slot.lock().ok())
        .and_then(|guard| guard.clone())
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

pub fn elicitation_questions(schema: &Value, message: &str) -> Vec<AskQuestionArg> {
    let mut questions = vec![AskQuestionArg {
        id: "action".into(),
        prompt: message.to_string(),
        label: Some("Elicitation".into()),
        mode: AskModeArg::Single,
        options: vec![
            AskOptionArg {
                value: "accept".into(),
                label: "Accept".into(),
                description: None,
                recommended: true,
            },
            AskOptionArg {
                value: "decline".into(),
                label: "Decline".into(),
                description: None,
                recommended: false,
            },
            AskOptionArg {
                value: "cancel".into(),
                label: "Cancel".into(),
                description: None,
                recommended: false,
            },
        ],
        allow_other: false,
    }];
    let Some(properties) = schema.get("properties").and_then(Value::as_object) else {
        return questions;
    };
    let required: Vec<&str> = schema
        .get("required")
        .and_then(Value::as_array)
        .map(|values| values.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    for (name, spec) in properties {
        let Some(spec) = spec.as_object() else {
            continue;
        };
        let kind = spec.get("type").and_then(Value::as_str).unwrap_or("string");
        let mut options = Vec::new();
        if let Some(values) = spec.get("enum").and_then(Value::as_array) {
            for value in values.iter().filter_map(Value::as_str) {
                options.push(AskOptionArg {
                    value: value.to_string(),
                    label: value.to_string(),
                    description: None,
                    recommended: false,
                });
            }
        } else if kind == "boolean" {
            options.push(AskOptionArg {
                value: "true".into(),
                label: "true".into(),
                description: None,
                recommended: false,
            });
            options.push(AskOptionArg {
                value: "false".into(),
                label: "false".into(),
                description: None,
                recommended: false,
            });
        }
        let description = spec
            .get("description")
            .and_then(Value::as_str)
            .or_else(|| spec.get("title").and_then(Value::as_str))
            .unwrap_or("");
        let marker = if required.contains(&name.as_str()) {
            "required"
        } else {
            "optional"
        };
        let label = format!("{name} ({kind}, {marker})");
        questions.push(AskQuestionArg {
            id: name.clone(),
            prompt: if description.is_empty() {
                label.clone()
            } else {
                format!("{description} [{label}]")
            },
            label: Some(label),
            mode: AskModeArg::Single,
            options,
            allow_other: true,
        });
    }
    questions
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct McpCatalog {
    #[serde(default)]
    pub prompts: Vec<McpCatalogEntry>,
    #[serde(default)]
    pub resources: Vec<McpCatalogEntry>,
}
