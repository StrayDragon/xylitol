//! Track-A tool-table freeze (c1900): phase + fingerprint + upsert helpers.
//!
//! Provider-visible `tools[]` becomes immutable after freeze until an explicit
//! re-gate (`/reload` idle, or resume/switch which **clears freeze** this wave).
//! Fingerprint continue-freeze needs persisted fingerprint (later wave).
//! Upsert = [`ToolSet::overlay_by_name`].

use std::time::Duration;

use sha2::{Digest, Sha256};

use super::ToolSet;

/// Wall-clock budget for first-turn / re-gate MCP settle before subset freeze.
///
/// Code-first (c1900); not YAML this wave. Per-server connect budget lives in
/// [`crate::infra::mcp::MCP_SERVER_CONNECT_TIMEOUT`].
pub const MCP_FIRST_TURN_GATE_TIMEOUT: Duration = Duration::from_secs(15);

/// Session-side tool-table phase (轨 A).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToolFreezePhase {
    /// Not frozen; settle may still install into the session tool table.
    #[default]
    Unfrozen,
    /// Gate started: waiting settle or timeout before freeze.
    Gating,
    /// Provider-visible table frozen; settle must not expand it.
    Frozen,
}

/// Fingerprint of a provider-visible tool table (ordered names + content digest).
///
/// Digest covers each tool's `name`, `description`, and canonical JSON schema in
/// name order — enough to detect add/remove/rename/schema drift on resume.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ToolTableFingerprint {
    pub names: Vec<String>,
    /// Lowercase hex SHA-256 of the canonical content stream.
    pub content_digest: String,
}

impl ToolTableFingerprint {
    /// Build a fingerprint from a [`ToolSet`] (iteration order = provider order).
    pub fn from_toolset(set: &ToolSet) -> Self {
        let mut names = Vec::new();
        let mut hasher = Sha256::new();
        for tool in set.iter() {
            let name = tool.name().to_string();
            let description = tool.description().to_string();
            let schema = tool.parameters_schema();
            let schema_canon = serde_json::to_string(&schema).unwrap_or_else(|_| "{}".into());
            names.push(name.clone());
            hasher.update(name.as_bytes());
            hasher.update([0xff]);
            hasher.update(description.as_bytes());
            hasher.update([0xff]);
            hasher.update(schema_canon.as_bytes());
            hasher.update([0xfe]);
        }
        Self {
            names,
            content_digest: hex_lower(hasher.finalize()),
        }
    }

    /// True when name order and content digest both match.
    pub fn matches(&self, other: &Self) -> bool {
        self == other
    }
}

fn hex_lower(bytes: impl AsRef<[u8]>) -> String {
    let b = bytes.as_ref();
    let mut out = String::with_capacity(b.len() * 2);
    for byte in b {
        use std::fmt::Write;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::protocol::ports::XyTool;

    struct Named {
        name: &'static str,
        description: &'static str,
        schema: serde_json::Value,
    }

    #[async_trait::async_trait]
    impl XyTool for Named {
        fn name(&self) -> &str {
            self.name
        }
        fn description(&self) -> &str {
            self.description
        }
        fn parameters_schema(&self) -> serde_json::Value {
            self.schema.clone()
        }
        async fn execute(
            &self,
            _: &crate::protocol::ports::XyToolCtx,
            _: serde_json::Value,
        ) -> Result<String, crate::protocol::error::XyToolError> {
            Ok("ok".into())
        }
    }

    fn tool(
        name: &'static str,
        description: &'static str,
        schema: serde_json::Value,
    ) -> Arc<dyn XyTool> {
        Arc::new(Named {
            name,
            description,
            schema,
        })
    }

    #[test]
    fn fingerprint_stable_for_same_set() {
        let set = ToolSet::from_iter(vec![
            tool("a", "da", serde_json::json!({"type": "object"})),
            tool(
                "b",
                "db",
                serde_json::json!({"type": "object", "properties": {}}),
            ),
        ]);
        let f1 = ToolTableFingerprint::from_toolset(&set);
        let f2 = ToolTableFingerprint::from_toolset(&set);
        assert!(f1.matches(&f2));
        assert_eq!(f1.names, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(f1.content_digest.len(), 64);
    }

    #[test]
    fn fingerprint_changes_on_schema_or_description() {
        let a = ToolSet::from_iter(vec![tool("t", "d1", serde_json::json!({"type": "object"}))]);
        let b = ToolSet::from_iter(vec![tool("t", "d2", serde_json::json!({"type": "object"}))]);
        let c = ToolSet::from_iter(vec![tool(
            "t",
            "d1",
            serde_json::json!({"type": "object", "required": ["x"]}),
        )]);
        let fa = ToolTableFingerprint::from_toolset(&a);
        assert!(!fa.matches(&ToolTableFingerprint::from_toolset(&b)));
        assert!(!fa.matches(&ToolTableFingerprint::from_toolset(&c)));
    }

    #[test]
    fn upsert_replaces_same_name_no_dup() {
        let base = ToolSet::from_iter(vec![tool(
            "mcp__fs__read",
            "old",
            serde_json::json!({"type": "object"}),
        )]);
        let incoming = ToolSet::from_iter(vec![
            tool(
                "mcp__fs__read",
                "new",
                serde_json::json!({"type": "object", "properties": {"path": {"type": "string"}}}),
            ),
            tool("mcp__fs__write", "w", serde_json::json!({"type": "object"})),
        ]);
        let merged = base.overlay_by_name(incoming);
        let names: Vec<_> = merged.iter().map(|t| t.name().to_string()).collect();
        assert_eq!(
            names,
            vec!["mcp__fs__read".to_string(), "mcp__fs__write".to_string()]
        );
        assert_eq!(merged.get("mcp__fs__read").unwrap().description(), "new");
    }

    #[test]
    fn freeze_table_core_then_armed_unique() {
        let core = vec![tool("read", "r", serde_json::json!({}))];
        let armed = vec![
            tool("mcp_x_t", "m", serde_json::json!({})),
            tool("read", "override-should-win", serde_json::json!({})),
        ];
        let frozen = ToolSet::rebuild_agent_tools(core, armed);
        let names: Vec<_> = frozen.iter().map(|t| t.name().to_string()).collect();
        assert_eq!(names, vec!["read".to_string(), "mcp_x_t".to_string()]);
        assert_eq!(
            frozen.get("read").unwrap().description(),
            "override-should-win"
        );
    }
}
