//! Security policy engine — built-in, config-gated.
//!
//! Provides [`SecurityEngine`] for checking tool calls against a declared
//! security policy (bash command regex, filesystem glob patterns, network
//! domain matching) and [`SecurityToolWrapper`] for transparently wrapping
//! any `adk_core::Tool` with a pre-execution check.
//!
//! Design principles:
//! - **Only-tighten**: merging rules always produces a more restrictive set.
//! - **Built-in before hooks**: security runs first; hooks can block but cannot
//!   override an already-blocked call.
//! - **No feature flag**: always compiled, gated by `security.enabled` config.

use std::sync::Arc;
use std::sync::atomic::{AtomicU16, Ordering};

use adk_core::{Result, Tool, ToolContext};
use async_trait::async_trait;
use regex::Regex;
use tracing::warn;

use crate::infra::config::types::SecurityConfig;

// ---------------------------------------------------------------------------
// SecurityVerdict
// ---------------------------------------------------------------------------

/// Result of a security policy check.
#[derive(Debug)]
pub(crate) enum SecurityVerdict {
    /// The operation is permitted.
    Allowed,
    /// The operation is blocked.
    Blocked { reason: String, rule: String },
}

// ---------------------------------------------------------------------------
// SecurityEngine
// ---------------------------------------------------------------------------

/// Thread-safe security engine with compiled regex/glob patterns.
#[derive(Clone)]
pub(crate) struct SecurityEngine {
    inner: Arc<SecurityEngineInner>,
}

struct SecurityEngineInner {
    enabled: bool,
    tool_allowlist: Vec<String>,
    bash_allowed: Vec<Regex>,
    bash_forbidden: Vec<Regex>,
    fs_allowed: Vec<String>,
    fs_forbidden: Vec<String>,
    network_allowed: Vec<String>,
    network_blocked: Vec<String>,
    max_subprocesses: u16,
    subprocess_count: AtomicU16,
}

impl SecurityEngine {
    /// Build an engine from the application security config.
    ///
    /// Invalid regex/glob patterns are logged with a warning and silently
    /// skipped — a bad pattern does not crash the process.
    pub(crate) fn new(config: &SecurityConfig) -> Self {
        let bash_allowed = compile_regex_list(&config.bash.allowed_paths, "bash.allowed_paths");
        let bash_forbidden =
            compile_regex_list(&config.bash.forbidden_patterns, "bash.forbidden_patterns");

        Self {
            inner: Arc::new(SecurityEngineInner {
                enabled: config.enabled,
                tool_allowlist: config.tool_allowlist.clone(),
                bash_allowed,
                bash_forbidden,
                fs_allowed: config.filesystem.allowed_patterns.clone(),
                fs_forbidden: config.filesystem.forbidden_patterns.clone(),
                network_allowed: config.network.allowed_domains.clone(),
                network_blocked: config.network.blocked_domains.clone(),
                max_subprocesses: config.resource_limits.max_subprocesses,
                subprocess_count: AtomicU16::new(0),
            }),
        }
    }

    /// Check whether a tool call with the given name and arguments is allowed.
    pub(crate) fn check_tool_call(&self, tool: &str, args: &serde_json::Value) -> SecurityVerdict {
        if !self.inner.enabled {
            return SecurityVerdict::Allowed;
        }

        // 1  Tool-level allowlist.
        if let verdict @ SecurityVerdict::Blocked { .. } = self.check_tool_allowlist(tool) {
            return verdict;
        }

        // 2  Tool-specific semantic checks.
        match tool {
            "bash" => self.check_bash(args),
            "read" | "write" | "edit" | "grep" | "find" | "ls" => self.check_file_tool(args),
            _ => SecurityVerdict::Allowed,
        }
    }

    // ── Helpers ──────────────────────────────────────────────────────

    fn check_tool_allowlist(&self, tool: &str) -> SecurityVerdict {
        let list = &self.inner.tool_allowlist;
        if list.is_empty() {
            return SecurityVerdict::Allowed;
        }
        if list.iter().any(|t| t == tool) {
            SecurityVerdict::Allowed
        } else {
            SecurityVerdict::Blocked {
                reason: format!("tool '{tool}' is not in the allowlist"),
                rule: "tool_allowlist".into(),
            }
        }
    }

    fn check_bash(&self, args: &serde_json::Value) -> SecurityVerdict {
        let Some(command) = args.get("command").and_then(|v| v.as_str()) else {
            return SecurityVerdict::Allowed;
        };

        // Forbidden patterns win — block immediately.
        for re in &self.inner.bash_forbidden {
            if re.is_match(command) {
                return SecurityVerdict::Blocked {
                    reason: format!("command matches forbidden pattern '{}'", re.as_str()),
                    rule: "bash.forbidden_patterns".into(),
                };
            }
        }

        // If allowed-patterns are set, at least one must match.
        if !self.inner.bash_allowed.is_empty()
            && !self
                .inner
                .bash_allowed
                .iter()
                .any(|re| re.is_match(command))
        {
            return SecurityVerdict::Blocked {
                reason: "command does not match any allowed pattern".into(),
                rule: "bash.allowed_paths".into(),
            };
        }

        SecurityVerdict::Allowed
    }

    fn check_file_tool(&self, args: &serde_json::Value) -> SecurityVerdict {
        let file_path = args.get("file_path").and_then(|v| v.as_str()).unwrap_or("");

        if file_path.is_empty() {
            return SecurityVerdict::Allowed;
        }

        let path = std::path::Path::new(file_path);

        // Check forbidden globs.
        if !self.inner.fs_forbidden.is_empty() {
            let forbidden = compile_glob_list(&self.inner.fs_forbidden);
            if forbidden.iter().any(|p| p.matches_path(path)) {
                return SecurityVerdict::Blocked {
                    reason: format!("path '{file_path}' matches forbidden pattern"),
                    rule: "filesystem.forbidden_patterns".into(),
                };
            }
        }

        // Check allowed globs (if any).
        if !self.inner.fs_allowed.is_empty() {
            let allowed = compile_glob_list(&self.inner.fs_allowed);
            if !allowed.iter().any(|p| p.matches_path(path)) {
                return SecurityVerdict::Blocked {
                    reason: format!("path '{file_path}' does not match any allowed pattern"),
                    rule: "filesystem.allowed_patterns".into(),
                };
            }
        }

        SecurityVerdict::Allowed
    }
}

impl SecurityEngine {
    /// Try to acquire a subprocess slot for bash execution.
    ///
    /// Returns a guard that releases the slot on drop. When at capacity,
    /// returns `Blocked`.
    pub(crate) fn acquire_subprocess(
        &self,
    ) -> std::result::Result<SubprocessGuard<'_>, SecurityVerdict> {
        let max = self.inner.max_subprocesses;
        if max == 0 {
            return Err(SecurityVerdict::Blocked {
                reason: "subprocess limit is 0 (disabled)".into(),
                rule: "resource_limits.max_subprocesses".into(),
            });
        }

        loop {
            let current = self.inner.subprocess_count.load(Ordering::Acquire);
            if current >= max {
                return Err(SecurityVerdict::Blocked {
                    reason: format!("subprocess limit reached ({current}/{max})",),
                    rule: "resource_limits.max_subprocesses".into(),
                });
            }
            if self
                .inner
                .subprocess_count
                .compare_exchange(current, current + 1, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                return Ok(SubprocessGuard(&self.inner.subprocess_count));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// SubprocessGuard
// ---------------------------------------------------------------------------

/// RAII guard that decrements the subprocess counter on drop.
pub(crate) struct SubprocessGuard<'a>(&'a AtomicU16);

impl Drop for SubprocessGuard<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Release);
    }
}

// ---------------------------------------------------------------------------
// SecurityToolWrapper
// ---------------------------------------------------------------------------

/// A `Tool` wrapper that runs a security check before delegating to the inner
/// tool.  When the check fails the call returns `Ok({"blocked": true, …})`
/// instead of executing — the LLM receives a clear explanation.
pub(crate) struct SecurityToolWrapper {
    inner: Arc<dyn Tool>,
    engine: SecurityEngine,
    tool_name: String,
}

impl std::fmt::Debug for SecurityToolWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecurityToolWrapper")
            .field("tool", &self.tool_name)
            .finish()
    }
}

impl SecurityToolWrapper {
    pub(crate) fn new(tool: Arc<dyn Tool>, engine: SecurityEngine) -> Self {
        let tool_name = tool.name().to_string();
        Self {
            inner: tool,
            engine,
            tool_name,
        }
    }
}

#[async_trait]
impl Tool for SecurityToolWrapper {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn description(&self) -> &str {
        self.inner.description()
    }

    fn parameters_schema(&self) -> Option<serde_json::Value> {
        self.inner.parameters_schema()
    }

    fn is_read_only(&self) -> bool {
        self.inner.is_read_only()
    }

    fn is_concurrency_safe(&self) -> bool {
        self.inner.is_concurrency_safe()
    }

    async fn execute(
        &self,
        ctx: Arc<dyn ToolContext>,
        args: serde_json::Value,
    ) -> Result<serde_json::Value> {
        match self.engine.check_tool_call(&self.tool_name, &args) {
            SecurityVerdict::Allowed => {
                // Acquire a subprocess slot for bash commands.
                if self.tool_name == "bash" {
                    match self.engine.acquire_subprocess() {
                        Ok(_guard) => self.inner.execute(ctx, args).await,
                        Err(verdict) => {
                            let (reason, rule) = match verdict {
                                SecurityVerdict::Blocked {
                                    ref reason,
                                    ref rule,
                                } => (reason.clone(), rule.clone()),
                                SecurityVerdict::Allowed => unreachable!(),
                            };
                            warn!(
                                tool = self.tool_name,
                                reason = reason,
                                rule = rule,
                                "Subprocess limit reached"
                            );
                            Ok(serde_json::json!({
                                "blocked": true,
                                "reason": reason,
                                "rule": rule,
                            }))
                        }
                    }
                } else {
                    self.inner.execute(ctx, args).await
                }
            }
            SecurityVerdict::Blocked { reason, rule } => {
                warn!(
                    tool = self.tool_name,
                    reason = reason,
                    rule = rule,
                    "Tool call blocked by security policy"
                );
                Ok(serde_json::json!({
                    "blocked": true,
                    "reason": reason,
                    "rule": rule,
                }))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Compile helpers
// ---------------------------------------------------------------------------

fn compile_regex_list(patterns: &[String], ctx: &str) -> Vec<Regex> {
    patterns
        .iter()
        .filter_map(|p| match Regex::new(p) {
            Ok(re) => Some(re),
            Err(e) => {
                warn!(context = ctx, pattern = p, error = %e, "Invalid regex pattern");
                None
            }
        })
        .collect()
}

fn compile_glob_list(patterns: &[String]) -> Vec<glob::Pattern> {
    patterns
        .iter()
        .filter_map(|p| match glob::Pattern::new(p) {
            Ok(g) => Some(g),
            Err(e) => {
                warn!(pattern = p, error = %e, "Invalid glob pattern");
                None
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::config::types::{
        BashSecurityConfig, FilesystemSecurityConfig, NetworkSecurityConfig, ResourceLimits,
    };

    // ── Test helpers ─────────────────────────────────────────────────

    fn minimal_config() -> SecurityConfig {
        SecurityConfig {
            enabled: true,
            tool_allowlist: vec![],
            bash: BashSecurityConfig {
                allowed_paths: vec![],
                forbidden_patterns: vec![],
                timeout_secs: 120,
            },
            filesystem: FilesystemSecurityConfig {
                allowed_patterns: vec![],
                forbidden_patterns: vec![],
            },
            network: NetworkSecurityConfig {
                allowed_domains: vec![],
                blocked_domains: vec![],
            },
            resource_limits: ResourceLimits::default(),
        }
    }

    fn make_engine(config: SecurityConfig) -> SecurityEngine {
        SecurityEngine::new(&config)
    }

    // ── Disabled = always allowed ────────────────────────────────────

    #[test]
    fn test_disabled_allows_everything() {
        let mut cfg = minimal_config();
        cfg.enabled = false;
        let engine = make_engine(cfg);
        assert!(matches!(
            engine.check_tool_call("bash", &serde_json::json!({"command": "rm -rf /"})),
            SecurityVerdict::Allowed
        ));
    }

    // ── Tool allowlist ───────────────────────────────────────────────

    #[test]
    fn test_tool_allowlist_allows_listed() {
        let mut cfg = minimal_config();
        cfg.tool_allowlist = vec!["read".into(), "write".into()];
        let engine = make_engine(cfg);
        assert!(matches!(
            engine.check_tool_call("read", &serde_json::json!({})),
            SecurityVerdict::Allowed
        ));
    }

    #[test]
    fn test_tool_allowlist_blocks_unlisted() {
        let mut cfg = minimal_config();
        cfg.tool_allowlist = vec!["read".into()];
        let engine = make_engine(cfg);
        match engine.check_tool_call("bash", &serde_json::json!({"command": "echo hi"})) {
            SecurityVerdict::Blocked { rule, .. } => {
                assert_eq!(rule, "tool_allowlist");
            }
            other => panic!("expected Blocked, got {other:?}"),
        }
    }

    #[test]
    fn test_empty_allowlist_allows_all() {
        let cfg = minimal_config();
        let engine = make_engine(cfg);
        assert!(matches!(
            engine.check_tool_call("anything", &serde_json::json!({})),
            SecurityVerdict::Allowed
        ));
    }

    // ── Bash forbidden patterns ──────────────────────────────────────

    #[test]
    fn test_bash_blocked_by_forbidden_pattern() {
        let mut cfg = minimal_config();
        cfg.bash.forbidden_patterns = vec!["rm\\s+-rf".into()];
        let engine = make_engine(cfg);
        match engine.check_tool_call("bash", &serde_json::json!({"command": "rm -rf /"})) {
            SecurityVerdict::Blocked { rule, .. } => {
                assert_eq!(rule, "bash.forbidden_patterns");
            }
            other => panic!("expected Blocked, got {other:?}"),
        }
    }

    #[test]
    fn test_bash_allowed_when_no_forbidden_match() {
        let mut cfg = minimal_config();
        cfg.bash.forbidden_patterns = vec!["rm".into()];
        let engine = make_engine(cfg);
        assert!(matches!(
            engine.check_tool_call("bash", &serde_json::json!({"command": "echo hello"})),
            SecurityVerdict::Allowed
        ));
    }

    // ── Bash allowed patterns ────────────────────────────────────────

    #[test]
    fn test_bash_requires_allowed_pattern_match() {
        let mut cfg = minimal_config();
        cfg.bash.allowed_paths = vec!["^(git|ls|cat)".into()];
        let engine = make_engine(cfg);
        assert!(matches!(
            engine.check_tool_call("bash", &serde_json::json!({"command": "git status"})),
            SecurityVerdict::Allowed
        ));
        match engine.check_tool_call("bash", &serde_json::json!({"command": "curl evil.com"})) {
            SecurityVerdict::Blocked { rule, .. } => {
                assert_eq!(rule, "bash.allowed_paths");
            }
            other => panic!("expected Blocked, got {other:?}"),
        }
    }

    // ── Filesystem forbidden patterns ────────────────────────────────

    #[test]
    fn test_fs_blocked_by_forbidden_glob() {
        let mut cfg = minimal_config();
        cfg.filesystem.forbidden_patterns = vec!["/etc/shadow*".into()];
        let engine = make_engine(cfg);
        match engine.check_tool_call("read", &serde_json::json!({"file_path": "/etc/shadow"})) {
            SecurityVerdict::Blocked { rule, .. } => {
                assert_eq!(rule, "filesystem.forbidden_patterns");
            }
            other => panic!("expected Blocked, got {other:?}"),
        }
    }

    #[test]
    fn test_fs_allowed_by_default_without_forbidden() {
        let cfg = minimal_config();
        let engine = make_engine(cfg);
        assert!(matches!(
            engine.check_tool_call("read", &serde_json::json!({"file_path": "/any/file"})),
            SecurityVerdict::Allowed
        ));
    }

    // ── Filesystem allowed patterns ──────────────────────────────────

    #[test]
    fn test_fs_allowed_pattern_required() {
        let mut cfg = minimal_config();
        cfg.filesystem.allowed_patterns = vec!["/home/**".into(), "/tmp/**".into()];
        let engine = make_engine(cfg);
        assert!(matches!(
            engine.check_tool_call(
                "write",
                &serde_json::json!({"file_path": "/home/user/file.txt"})
            ),
            SecurityVerdict::Allowed
        ));
        assert!(matches!(
            engine.check_tool_call("write", &serde_json::json!({"file_path": "/tmp/x"})),
            SecurityVerdict::Allowed
        ));
        match engine.check_tool_call("write", &serde_json::json!({"file_path": "/etc/passwd"})) {
            SecurityVerdict::Blocked { rule, .. } => {
                assert_eq!(rule, "filesystem.allowed_patterns");
            }
            other => panic!("expected Blocked, got {other:?}"),
        }
    }

    // ── Tool-specific routing ────────────────────────────────────────

    #[test]
    fn test_fs_check_applies_to_all_file_tools() {
        let mut cfg = minimal_config();
        cfg.filesystem.forbidden_patterns = vec!["/etc/**".into()];
        let engine = make_engine(cfg);
        for tool in &["read", "write", "edit", "grep", "find", "ls"] {
            match engine.check_tool_call(tool, &serde_json::json!({"file_path": "/etc/passwd"})) {
                SecurityVerdict::Blocked { .. } => {} // expected
                other => panic!("{tool} expected Blocked, got {other:?}"),
            }
        }
    }

    #[test]
    fn test_non_matching_tool_is_allowed() {
        let engine = make_engine(minimal_config());
        // Unknown tools are allowed by default (tool_allowlist not set)
        assert!(matches!(
            engine.check_tool_call("unknown_tool", &serde_json::json!({})),
            SecurityVerdict::Allowed
        ));
    }

    // ── Invalid patterns do not crash ────────────────────────────────

    #[test]
    fn test_invalid_regex_skipped() {
        let mut cfg = minimal_config();
        cfg.bash.forbidden_patterns = vec!["[invalid".into(), "rm".into()];
        let engine = make_engine(cfg);
        match engine.check_tool_call("bash", &serde_json::json!({"command": "rm -rf /"})) {
            SecurityVerdict::Blocked { rule, .. } => {
                // The valid pattern still works
                assert_eq!(rule, "bash.forbidden_patterns");
            }
            other => panic!("expected Blocked, got {other:?}"),
        }
    }

    #[test]
    fn test_invalid_glob_skipped() {
        let mut cfg = minimal_config();
        cfg.filesystem.forbidden_patterns = vec!["[".into()]; // invalid glob
        let engine = make_engine(cfg);
        // No valid patterns → nothing blocked
        assert!(matches!(
            engine.check_tool_call("read", &serde_json::json!({"file_path": "/etc/passwd"})),
            SecurityVerdict::Allowed
        ));
    }

    // ── SecurityToolWrapper ──────────────────────────────────────────

    #[tokio::test]
    async fn test_wrapper_blocks_and_returns_blocked_json() {
        use crate::agent::tools::read::ReadTool;
        use std::sync::Arc;

        let mut cfg = minimal_config();
        cfg.filesystem.forbidden_patterns = vec!["/etc/**".into()];
        let engine = SecurityEngine::new(&cfg);

        let wrapper = SecurityToolWrapper::new(Arc::new(ReadTool), engine);
        let ctx = crate::agent::tools::patch::mock_context();
        let result = wrapper
            .execute(ctx, serde_json::json!({"file_path": "/etc/passwd"}))
            .await
            .unwrap();
        assert_eq!(result["blocked"], true);
        assert!(result["reason"].as_str().unwrap().contains("forbidden"));
    }

    #[tokio::test]
    async fn test_wrapper_allows_and_forwards() {
        use crate::agent::tools::read::ReadTool;
        use std::sync::Arc;

        let cfg = minimal_config(); // no restrictions
        let engine = SecurityEngine::new(&cfg);

        let wrapper = SecurityToolWrapper::new(Arc::new(ReadTool), engine);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        tokio::fs::write(&path, "hello").await.unwrap();

        let ctx = crate::agent::tools::patch::mock_context();
        let result = wrapper
            .execute(
                ctx,
                serde_json::json!({"file_path": path.to_str().unwrap()}),
            )
            .await
            .unwrap();
        assert_eq!(result["content"], "hello");
    }
}
