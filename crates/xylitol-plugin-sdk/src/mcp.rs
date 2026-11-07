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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct McpCatalog {
    #[serde(default)]
    pub prompts: Vec<McpCatalogEntry>,
    #[serde(default)]
    pub resources: Vec<McpCatalogEntry>,
}
