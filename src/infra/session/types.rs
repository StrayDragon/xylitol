#![allow(dead_code)] // WIP: not yet integrated into main flow

use std::collections::HashMap;
use std::path::PathBuf;

use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};

/// A snapshot ID (UUID v4).
pub type SnapshotId = String;

/// A project hash (SHA-256 hex).
pub type ProjectHash = String;

/// Convenience newtype for Unix-millisecond timestamps (works cleanly with rmp-serde).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TimestampMillis(i64);

impl TimestampMillis {
    pub fn now() -> Self {
        Self(Utc::now().timestamp_millis())
    }

    pub fn from_datetime(dt: DateTime<Utc>) -> Self {
        Self(dt.timestamp_millis())
    }

    pub fn to_datetime(self) -> DateTime<Utc> {
        Utc.timestamp_millis_opt(self.0).unwrap()
    }

    pub fn millis(self) -> i64 {
        self.0
    }

    /// Subtract a number of milliseconds, returning None on underflow.
    pub fn checked_sub_millis(self, ms: i64) -> Option<Self> {
        self.0.checked_sub(ms).map(Self)
    }
}

impl From<DateTime<Utc>> for TimestampMillis {
    fn from(dt: DateTime<Utc>) -> Self {
        Self::from_datetime(dt)
    }
}

impl From<TimestampMillis> for DateTime<Utc> {
    fn from(ts: TimestampMillis) -> Self {
        ts.to_datetime()
    }
}

/// Immutable session snapshot — the core data structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: SnapshotId,
    pub created: TimestampMillis,
    pub parent_snapshot_id: Option<SnapshotId>,
    pub meta: SnapshotMeta,
    pub conversation: Vec<ConversationTurn>,
    pub project_cognition: ProjectCognition,
    pub tool_call_log: Vec<ToolCallSummary>,
    pub config_fingerprint: ConfigFingerprint,
}

/// Metadata attached to every snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMeta {
    pub project_root: PathBuf,
    pub project_hash: String,
    pub model_id: String,
    pub tags: Vec<String>,
}

/// Project-level cognition — agent's "long-term memory".
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectCognition {
    pub code_summaries: HashMap<String, CodeSummary>,
    pub codebase_graph: CodebaseGraph,
    /// DAP debugger state; always `None` until DAP dev resumes (paused 2026-05-17).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debugger_state: Option<DebuggerState>,
}

impl ProjectCognition {
    pub fn empty() -> Self {
        Self {
            code_summaries: HashMap::new(),
            codebase_graph: CodebaseGraph {
                nodes: vec![],
                edges: vec![],
            },
            debugger_state: None,
        }
    }
}

/// LSP-derived summary for a single module/file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSummary {
    pub summary: String,
    pub symbols: Vec<String>,
    pub last_indexed: TimestampMillis,
    /// Set to `true` when project_hash has changed since this summary was created.
    pub stale: bool,
}

/// Dependency graph of the codebase (nodes = symbols, edges = dependency relations).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CodebaseGraph {
    pub nodes: Vec<String>,
    pub edges: Vec<(String, String)>,
}

/// Debugger state — reserved for DAP integration (paused).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebuggerState;

/// A single turn in the conversation history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    pub role: ConversationRole,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallSummary>>,
    pub timestamp: TimestampMillis,
    /// When `true`, this turn has been superseded (e.g., by compaction).
    pub deprecated: bool,
}

/// Role of a conversation participant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConversationRole {
    User,
    Assistant,
    System,
    Tool,
}

/// Summarised tool invocation — keeps only essential metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallSummary {
    pub tool: String,
    pub params: serde_json::Value,
    pub result_summary: String,
    pub tokens_consumed: u32,
}

/// Fingerprint of the agent configuration at snapshot time.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConfigFingerprint {
    pub features: Vec<String>,
    pub config_hash: String,
}
