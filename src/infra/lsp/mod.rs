//! LSP integration layer — wraps lspz agent-sdk with configurable init strategies,
//! lifecycle management, and TOON text compression output.
//!
//! Gated behind `feature = "infra-lsp"`.

use std::collections::HashMap;

use lspz::agent_sdk::AgentHandle;
use serde_json::Value;

// ── Config types ─────────────────────────────────────────────────────────────

/// LSP initialization strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InitStrategy {
    /// Start LSP server on first query (default).
    #[default]
    Lazy,
    /// Start all registered LSP servers immediately during initialization.
    PreInit,
    /// Start on first query with workspace pre-warming.
    OnDemandWarm,
}

/// Registered LSP backend descriptor.
#[derive(Debug, Clone)]
pub struct LspBackend {
    pub language: String,
    pub backend: String,
    pub workspace_root: Option<String>,
}

/// LSP pool configuration.
#[derive(Debug, Clone, Default)]
pub struct LspConfig {
    pub backends: Vec<LspBackend>,
    pub strategy: InitStrategy,
    pub compression: bool,
}

// ── LspPool ──────────────────────────────────────────────────────────────────

/// Pool of LSP sessions with configurable initialization strategy.
///
/// Manages LSP server subprocesses via `AgentHandle`. Sessions are created
/// according to `InitStrategy` and shut down when the pool is dropped.
pub struct LspPool {
    handles: HashMap<String, AgentHandle>,
    backends: HashMap<String, LspBackend>,
    strategy: InitStrategy,
    compression: bool,
}

impl std::fmt::Debug for LspPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LspPool")
            .field("registered_languages", &self.backends.len())
            .field("active_handles", &self.handles.len())
            .field("strategy", &self.strategy)
            .field("compression", &self.compression)
            .finish()
    }
}

impl LspPool {
    /// Create a new empty pool. Call [`init`](Self::init) before making queries.
    pub fn new(config: LspConfig) -> Self {
        let backends = config
            .backends
            .into_iter()
            .map(|b| (b.language.clone(), b))
            .collect();
        Self {
            handles: HashMap::new(),
            backends,
            strategy: config.strategy,
            compression: config.compression,
        }
    }

    /// Initialize the pool according to the configured strategy.
    ///
    /// For `PreInit`, this spawns all LSP server subprocesses.
    /// For `Lazy` and `OnDemandWarm`, this is a no-op (handles are created on first query).
    pub async fn init(&mut self) -> Result<(), anyhow::Error> {
        if self.strategy == InitStrategy::PreInit {
            let languages: Vec<String> = self.backends.keys().cloned().collect();
            for lang in &languages {
                self.ensure_handle(lang).await?;
            }
        }
        Ok(())
    }

    /// Get or create an `AgentHandle` for the given language.
    async fn ensure_handle(&mut self, language: &str) -> Result<&mut AgentHandle, anyhow::Error> {
        use std::collections::hash_map::Entry;

        match self.handles.entry(language.to_owned()) {
            Entry::Occupied(e) => Ok(e.into_mut()),
            Entry::Vacant(e) => {
                let cfg = self
                    .backends
                    .get(language)
                    .ok_or_else(|| anyhow::anyhow!("no LSP backend registered for '{language}'"))?;

                let mut builder = AgentHandle::builder()
                    .backend(&cfg.backend)
                    .language(language)
                    .enable_compression(self.compression);
                if let Some(root) = &cfg.workspace_root {
                    builder = builder.workspace_root(root);
                }

                let mut handle = builder.start().await?;
                tracing::info!(language, backend = %cfg.backend, "LSP session started");

                if self.strategy == InitStrategy::OnDemandWarm {
                    // Pre-warm: send a workspace/symbol query to populate caches.
                    // Best-effort — ignore failures during warmup.
                    if let Err(e) = handle.get_workspace_symbols("").await {
                        tracing::debug!(language, error = %e, "LSP warmup query failed");
                    }
                }

                Ok(e.insert(handle))
            }
        }
    }

    // ── Query methods — 10+ ────────────────────────────────────────────────

    /// Get diagnostics for a file.
    pub async fn get_diagnostics(
        &mut self,
        uri: &str,
        language: &str,
    ) -> Result<String, anyhow::Error> {
        self.ensure_handle(language)
            .await?
            .get_diagnostics(uri)
            .await
    }

    /// Get completions at a cursor position.
    pub async fn get_completions(
        &mut self,
        uri: &str,
        language: &str,
        line: u32,
        character: u32,
    ) -> Result<String, anyhow::Error> {
        self.ensure_handle(language)
            .await?
            .get_completions(uri, line, character)
            .await
    }

    /// Get hover information at a cursor position.
    pub async fn get_hover(
        &mut self,
        uri: &str,
        language: &str,
        line: u32,
        character: u32,
    ) -> Result<String, anyhow::Error> {
        self.ensure_handle(language)
            .await?
            .get_hover(uri, line, character)
            .await
    }

    /// Get document symbols.
    pub async fn get_symbols(
        &mut self,
        uri: &str,
        language: &str,
    ) -> Result<String, anyhow::Error> {
        self.ensure_handle(language).await?.get_symbols(uri).await
    }

    /// Get references at a cursor position.
    pub async fn get_references(
        &mut self,
        uri: &str,
        language: &str,
        line: u32,
        character: u32,
    ) -> Result<String, anyhow::Error> {
        self.ensure_handle(language)
            .await?
            .get_references(uri, line, character)
            .await
    }

    /// Get definition location of a symbol.
    pub async fn get_definition(
        &mut self,
        uri: &str,
        language: &str,
        line: u32,
        character: u32,
    ) -> Result<String, anyhow::Error> {
        self.ensure_handle(language)
            .await?
            .get_definition(uri, line, character)
            .await
    }

    /// Get implementation locations of a symbol.
    pub async fn get_implementation(
        &mut self,
        uri: &str,
        language: &str,
        line: u32,
        character: u32,
    ) -> Result<String, anyhow::Error> {
        self.ensure_handle(language)
            .await?
            .get_implementation(uri, line, character)
            .await
    }

    /// Get type definition location of a symbol.
    pub async fn get_type_definition(
        &mut self,
        uri: &str,
        language: &str,
        line: u32,
        character: u32,
    ) -> Result<String, anyhow::Error> {
        self.ensure_handle(language)
            .await?
            .get_type_definition(uri, line, character)
            .await
    }

    /// Query workspace symbols matching a search term.
    pub async fn get_workspace_symbols(
        &mut self,
        language: &str,
        query: &str,
    ) -> Result<String, anyhow::Error> {
        self.ensure_handle(language)
            .await?
            .get_workspace_symbols(query)
            .await
    }

    /// Get workspace diagnostics for a file.
    pub async fn get_workspace_diagnostics(
        &mut self,
        uri: &str,
        language: &str,
    ) -> Result<String, anyhow::Error> {
        self.ensure_handle(language)
            .await?
            .get_workspace_diagnostics(uri)
            .await
    }

    /// Send a raw LSP request bypassing the interceptor chain.
    pub async fn send_raw(
        &mut self,
        language: &str,
        method: &str,
        params: Value,
    ) -> Result<String, anyhow::Error> {
        self.ensure_handle(language)
            .await?
            .send_raw(method, params)
            .await
    }

    // ── File sync notifications — 3 ────────────────────────────────────────

    /// Notify the LSP server that a file's content has changed.
    pub async fn notify_change(
        &mut self,
        uri: &str,
        language: &str,
        content: &str,
    ) -> Result<(), anyhow::Error> {
        self.ensure_handle(language)
            .await?
            .notify_change(uri, content)
            .await
    }

    /// Notify the LSP server that a file has been closed.
    pub async fn notify_close(&mut self, uri: &str, language: &str) -> Result<(), anyhow::Error> {
        self.ensure_handle(language).await?.notify_close(uri).await
    }

    /// Notify the LSP server that a file has been saved.
    pub async fn notify_save(&mut self, uri: &str, language: &str) -> Result<(), anyhow::Error> {
        self.ensure_handle(language).await?.notify_save(uri).await
    }

    // ── Refactoring operations — 3 ─────────────────────────────────────────

    /// Rename a symbol at the given position.
    pub async fn rename(
        &mut self,
        uri: &str,
        language: &str,
        line: u32,
        character: u32,
        new_name: &str,
    ) -> Result<String, anyhow::Error> {
        self.ensure_handle(language)
            .await?
            .rename(uri, line, character, new_name)
            .await
    }

    /// Get code actions for a position in a file.
    #[allow(clippy::too_many_arguments)]
    pub async fn code_action(
        &mut self,
        uri: &str,
        language: &str,
        line: u32,
        character: u32,
        diagnostics: Option<Vec<Value>>,
        only: Option<Vec<String>>,
    ) -> Result<String, anyhow::Error> {
        self.ensure_handle(language)
            .await?
            .code_action(uri, line, character, diagnostics, only)
            .await
    }

    /// Format a file.
    pub async fn formatting(
        &mut self,
        uri: &str,
        language: &str,
        options: Option<Value>,
    ) -> Result<String, anyhow::Error> {
        self.ensure_handle(language)
            .await?
            .formatting(uri, options)
            .await
    }

    // ── Lifecycle ──────────────────────────────────────────────────────────

    /// Shut down all LSP server sessions gracefully.
    pub async fn shutdown_all(mut self) -> Result<(), anyhow::Error> {
        let handles: Vec<(String, AgentHandle)> = self.handles.drain().collect();
        for (language, handle) in handles {
            if let Err(e) = handle.shutdown().await {
                tracing::warn!(language, error = %e, "LSP shutdown failed");
            }
        }
        Ok(())
    }

    /// Insert a pre-constructed [`AgentHandle`] (for testing).
    #[cfg(test)]
    pub(crate) fn insert_handle(&mut self, language: &str, handle: AgentHandle) {
        // Also ensure a backend entry exists so pool invariants hold.
        self.backends.entry(language.to_owned()).or_insert_with(|| {
            tracing::warn!(language, "insert_handle without registered backend");
            LspBackend {
                language: language.to_owned(),
                backend: String::new(),
                workspace_root: None,
            }
        });
        self.handles.insert(language.to_owned(), handle);
    }

    /// Number of active (started) LSP sessions.
    pub fn num_active(&self) -> usize {
        self.handles.len()
    }

    /// Number of registered backends.
    pub fn num_registered(&self) -> usize {
        self.backends.len()
    }

    /// List languages with registered backends.
    pub fn registered_languages(&self) -> Vec<&str> {
        self.backends.keys().map(|s| s.as_str()).collect()
    }

    /// List languages with active sessions.
    pub fn active_languages(&self) -> Vec<&str> {
        self.handles.keys().map(|s| s.as_str()).collect()
    }

    // ── Compression helpers — 2 ────────────────────────────────────────────

    /// Compress standard LSP output into compact JSON format.
    pub fn compress(raw_json: &str) -> Result<String, anyhow::Error> {
        AgentHandle::compress(raw_json)
    }

    /// Expand compact JSON back to standard LSP format.
    pub fn inflate(compressed_json: &str) -> Result<String, anyhow::Error> {
        AgentHandle::inflate(compressed_json)
    }

    /// Convert compact diagnostics JSON to TOON text format.
    ///
    /// The input should be output from a `get_diagnostics` call with compression enabled.
    pub fn diagnostics_to_toon(compact_json: &str) -> Result<String, anyhow::Error> {
        let value: Value = serde_json::from_str(compact_json)?;
        let diags: lspz::codec::compact::CompactDiagnostics = serde_json::from_value(value)?;
        Ok(lspz::codec::toon::diagnostics_to_toon(&diags))
    }

    /// Convert compact completions JSON to TOON text format.
    pub fn completions_to_toon(compact_json: &str) -> Result<String, anyhow::Error> {
        let value: Value = serde_json::from_str(compact_json)?;
        Ok(lspz::codec::toon::completions_to_toon(&value)?)
    }

    /// Convert compact hover JSON to TOON text format.
    pub fn hover_to_toon(compact_json: &str) -> Result<String, anyhow::Error> {
        let value: Value = serde_json::from_str(compact_json)?;
        Ok(lspz::codec::toon::hover_to_toon(&value)?)
    }

    /// Convert compact symbol JSON to TOON text format.
    pub fn symbols_to_toon(compact_json: &str) -> Result<String, anyhow::Error> {
        let value: Value = serde_json::from_str(compact_json)?;
        Ok(lspz::codec::toon::symbols_to_toon(&value)?)
    }

    /// Convert compact location JSON to TOON text format.
    pub fn locations_to_toon(compact_json: &str) -> Result<String, anyhow::Error> {
        let value: Value = serde_json::from_str(compact_json)?;
        Ok(lspz::codec::toon::locations_to_toon(&value)?)
    }
}

impl Drop for LspPool {
    fn drop(&mut self) {
        let count = self.handles.len();
        if count > 0 {
            tracing::debug!(count, "Dropping LspPool with active sessions");
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Config / strategy tests ────────────────────────────────────────────

    #[test]
    fn test_default_strategy_is_lazy() {
        let config = LspConfig::default();
        assert_eq!(config.strategy, InitStrategy::Lazy);
    }

    #[test]
    fn test_new_pool_empty() {
        let config = LspConfig::default();
        let pool = LspPool::new(config);
        assert_eq!(pool.num_registered(), 0);
        assert_eq!(pool.num_active(), 0);
    }

    #[test]
    fn test_register_backends() {
        let config = LspConfig {
            backends: vec![
                LspBackend {
                    language: "rust".into(),
                    backend: "rust-analyzer".into(),
                    workspace_root: None,
                },
                LspBackend {
                    language: "python".into(),
                    backend: "basedpyright".into(),
                    workspace_root: Some("/project".into()),
                },
            ],
            strategy: InitStrategy::Lazy,
            compression: true,
        };
        let pool = LspPool::new(config);
        assert_eq!(pool.num_registered(), 2);
        let langs = pool.registered_languages();
        assert!(langs.contains(&"rust"));
        assert!(langs.contains(&"python"));
    }

    #[test]
    fn test_ensure_handle_unknown_language() {
        let mut pool = LspPool::new(LspConfig {
            backends: vec![LspBackend {
                language: "rust".into(),
                backend: "rust-analyzer".into(),
                workspace_root: None,
            }],
            strategy: InitStrategy::Lazy,
            compression: false,
        });

        let rt = tokio::runtime::Runtime::new().unwrap();
        let err = rt.block_on(pool.ensure_handle("python")).unwrap_err();
        assert!(
            err.to_string().contains("no LSP backend registered"),
            "{err}"
        );
    }

    #[test]
    fn test_strategy_debug() {
        assert_eq!(format!("{:?}", InitStrategy::Lazy), "Lazy");
        assert_eq!(format!("{:?}", InitStrategy::PreInit), "PreInit");
        assert_eq!(format!("{:?}", InitStrategy::OnDemandWarm), "OnDemandWarm");
    }

    // ── Compression helpers ────────────────────────────────────────────────

    #[test]
    fn test_compress_roundtrip() {
        let raw = serde_json::json!({
            "uri": "file:///test.rs",
            "diagnostics": [{
                "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 1, "character": 0 } },
                "severity": 1,
                "message": "test msg",
            }],
        });
        let raw_str = raw.to_string();
        let compact = LspPool::compress(&raw_str).unwrap();
        let expanded = LspPool::inflate(&compact).unwrap();
        let expanded_val: Value = serde_json::from_str(&expanded).unwrap();
        assert_eq!(expanded_val["diagnostics"][0]["message"], "test msg");
    }

    #[test]
    fn test_compress_invalid_json() {
        let err = LspPool::compress("not json").unwrap_err();
        assert!(err.to_string().contains("expected"), "{err}");
    }

    // ── TOON conversion (compact JSON → TOON text) ────────────────────────

    #[test]
    fn test_diagnostics_to_toon() {
        let compact = serde_json::json!({
            "version": 1,
            "uri": "file:///src/main.rs",
            "diagnostics": [{
                "m": "unused variable",
                "s": "W",
                "r": [[10, 5, 10, 15]],
                "c": "unused_vars",
                "t": null,
                "n": 1,
            }],
        });
        let toon = LspPool::diagnostics_to_toon(&compact.to_string()).unwrap();
        assert!(toon.contains("uri: file:///src/main.rs"));
        assert!(toon.contains("warning"));
        assert!(toon.contains("unused variable"));
    }

    #[test]
    fn test_completions_to_toon() {
        let compact = serde_json::json!({
            "version": 1,
            "incomplete": false,
            "items": [
                {"l": "push", "k": "F", "d": "fn push()"},
                {"l": "pop", "k": "F", "d": "fn pop()"},
            ]
        });
        let toon = LspPool::completions_to_toon(&compact.to_string()).unwrap();
        assert!(toon.contains("completions[2]{"));
        assert!(toon.contains("push,function"));
    }

    #[test]
    fn test_hover_to_toon() {
        let compact = serde_json::json!({
            "c": {"k": "m", "v": "**fn main** — entry point"}
        });
        let toon = LspPool::hover_to_toon(&compact.to_string()).unwrap();
        assert!(toon.contains("kind: markdown"));
        assert!(toon.contains("entry point"));
    }

    #[test]
    fn test_symbols_to_toon() {
        let compact = serde_json::json!({
            "version": 1,
            "items": [
                {"n": "main", "k": "u", "r": {"s": {"l": 0, "c": 0}, "e": {"l": 1, "c": 0}}}
            ]
        });
        let toon = LspPool::symbols_to_toon(&compact.to_string()).unwrap();
        assert!(toon.contains("main,function"));
    }

    #[test]
    fn test_locations_to_toon() {
        let compact = serde_json::json!({
            "version": 1,
            "uris": ["file:///src/lib.rs"],
            "items": [
                {"u": 0, "r": {"s": {"l": 5, "c": 0}, "e": {"l": 5, "c": 10}}}
            ]
        });
        let toon = LspPool::locations_to_toon(&compact.to_string()).unwrap();
        assert!(toon.contains("uris[1]:"));
        assert!(toon.contains("locations[1]{uri,range}:"));
    }

    #[test]
    fn test_invalid_toon_input() {
        let err = LspPool::diagnostics_to_toon("not json").unwrap_err();
        assert!(err.to_string().contains("expected"), "{err}");
    }

    // ── Pool info ──────────────────────────────────────────────────────────

    #[test]
    fn test_pool_debug_no_backends() {
        let pool = LspPool::new(LspConfig::default());
        let debug = format!("{:?}", pool);
        assert!(debug.contains("registered_languages: 0"));
        assert!(debug.contains("active_handles: 0"));
    }

    // ── Mock transport tests ──────────────────────────────────────────────

    use lspz::codec::json_rpc::LspMessage;
    use lspz::mcp::LspSession;
    use lspz::transport::mock::MockTransport;

    /// Create a temp file and return (file:// URI, path).
    fn temp_file(content: &str) -> (String, String) {
        static COUNTER: std::sync::atomic::AtomicU16 = std::sync::atomic::AtomicU16::new(0);
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let path = format!("/tmp/xylitol-lsp-test-{id}.rs");
        std::fs::write(&path, content).unwrap();
        (format!("file://{path}"), path)
    }

    /// Build a mock AgentHandle backed by a MockTransport.
    fn mock_handle(responses: Vec<LspMessage>, compression: bool) -> AgentHandle {
        let mock = MockTransport::new();
        for msg in responses {
            mock.push_message(&msg).unwrap();
        }
        let session = LspSession::with_transport(Box::new(mock));
        AgentHandle::new(session, "rust".into(), compression)
    }

    #[tokio::test]
    async fn test_mock_get_diagnostics() {
        let diag = LspMessage::Notification {
            method: "textDocument/publishDiagnostics".into(),
            params: serde_json::json!({
                "uri": "file:///test.rs",
                "diagnostics": [{
                    "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 1, "character": 0 } },
                    "severity": 1,
                    "message": "mock error",
                }],
            }),
        };

        let handle = mock_handle(vec![diag], false);
        let mut pool = LspPool::new(LspConfig {
            backends: vec![LspBackend {
                language: "rust".into(),
                backend: "rust-analyzer".into(),
                workspace_root: None,
            }],
            strategy: InitStrategy::Lazy,
            compression: false,
        });
        pool.insert_handle("rust", handle);

        let (uri, _path) = temp_file("fn main() {}");
        let result = pool.get_diagnostics(&uri, "rust").await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["diagnostics"][0]["message"], "mock error");
    }

    #[tokio::test]
    async fn test_mock_get_completions() {
        let resp = LspMessage::Response {
            id: 1,
            result: Some(serde_json::json!({
                "items": [
                    { "label": "fn", "kind": 14 },
                    { "label": "for", "kind": 14 },
                ],
            })),
            error: None,
        };

        let handle = mock_handle(vec![resp], false);
        let mut pool = LspPool::new(LspConfig {
            backends: vec![LspBackend {
                language: "rust".into(),
                backend: "rust-analyzer".into(),
                workspace_root: None,
            }],
            strategy: InitStrategy::Lazy,
            compression: false,
        });
        pool.insert_handle("rust", handle);

        let (uri, _path) = temp_file("fn main() {}");
        let result = pool.get_completions(&uri, "rust", 0, 0).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["items"][0]["label"], "fn");
    }

    #[tokio::test]
    async fn test_mock_get_hover() {
        let resp = LspMessage::Response {
            id: 1,
            result: Some(serde_json::json!({
                "contents": {
                    "kind": "markdown",
                    "value": "**fn main** — entry point",
                },
            })),
            error: None,
        };

        let handle = mock_handle(vec![resp], false);
        let mut pool = LspPool::new(LspConfig {
            backends: vec![LspBackend {
                language: "rust".into(),
                backend: "rust-analyzer".into(),
                workspace_root: None,
            }],
            strategy: InitStrategy::Lazy,
            compression: false,
        });
        pool.insert_handle("rust", handle);

        let (uri, _path) = temp_file("fn main() {}");
        let result = pool.get_hover(&uri, "rust", 0, 0).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["contents"]["value"], "**fn main** — entry point");
    }

    #[tokio::test]
    async fn test_mock_get_symbols() {
        let resp = LspMessage::Response {
            id: 1,
            result: Some(serde_json::json!([
                { "name": "main", "kind": 12 },
            ])),
            error: None,
        };

        let handle = mock_handle(vec![resp], false);
        let mut pool = LspPool::new(LspConfig {
            backends: vec![LspBackend {
                language: "rust".into(),
                backend: "rust-analyzer".into(),
                workspace_root: None,
            }],
            strategy: InitStrategy::Lazy,
            compression: false,
        });
        pool.insert_handle("rust", handle);

        let (uri, _path) = temp_file("fn main() {}");
        let result = pool.get_symbols(&uri, "rust").await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed[0]["name"], "main");
    }

    #[tokio::test]
    async fn test_mock_get_definition() {
        let resp = LspMessage::Response {
            id: 1,
            result: Some(serde_json::json!({
                "uri": "file:///src/lib.rs",
                "range": { "start": { "line": 1, "character": 0 }, "end": { "line": 1, "character": 10 } },
            })),
            error: None,
        };

        let handle = mock_handle(vec![resp], false);
        let mut pool = LspPool::new(LspConfig {
            backends: vec![LspBackend {
                language: "rust".into(),
                backend: "rust-analyzer".into(),
                workspace_root: None,
            }],
            strategy: InitStrategy::Lazy,
            compression: false,
        });
        pool.insert_handle("rust", handle);

        let (uri, _path) = temp_file("fn main() {}");
        let result = pool.get_definition(&uri, "rust", 0, 0).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["uri"], "file:///src/lib.rs");
    }

    #[tokio::test]
    async fn test_mock_get_references() {
        let resp = LspMessage::Response {
            id: 1,
            result: Some(serde_json::json!([
                { "uri": "file:///lib.rs", "range": { "start": { "line": 5, "character": 0 }, "end": { "line": 5, "character": 1 } } }
            ])),
            error: None,
        };

        let handle = mock_handle(vec![resp], false);
        let mut pool = LspPool::new(LspConfig {
            backends: vec![LspBackend {
                language: "rust".into(),
                backend: "rust-analyzer".into(),
                workspace_root: None,
            }],
            strategy: InitStrategy::Lazy,
            compression: false,
        });
        pool.insert_handle("rust", handle);

        let (uri, _path) = temp_file("fn main() {}");
        let result = pool.get_references(&uri, "rust", 0, 0).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed[0]["uri"], "file:///lib.rs");
    }

    #[tokio::test]
    async fn test_mock_get_workspace_symbols() {
        let resp = LspMessage::Response {
            id: 1,
            result: Some(serde_json::json!([
                { "name": "main", "kind": 12, "location": {
                    "uri": "file:///src/main.rs",
                    "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 1 } },
                }},
            ])),
            error: None,
        };

        let handle = mock_handle(vec![resp], false);
        let mut pool = LspPool::new(LspConfig {
            backends: vec![LspBackend {
                language: "rust".into(),
                backend: "rust-analyzer".into(),
                workspace_root: None,
            }],
            strategy: InitStrategy::Lazy,
            compression: false,
        });
        pool.insert_handle("rust", handle);

        let result = pool.get_workspace_symbols("rust", "main").await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed[0]["name"], "main");
    }

    #[tokio::test]
    async fn test_mock_rename() {
        let resp = LspMessage::Response {
            id: 1,
            result: Some(serde_json::json!({
                "changes": {
                    "file:///test.rs": [{
                        "range": { "start": { "line": 0, "character": 3 }, "end": { "line": 0, "character": 7 } },
                        "newText": "new_name",
                    }]
                }
            })),
            error: None,
        };

        let handle = mock_handle(vec![resp], false);
        let mut pool = LspPool::new(LspConfig {
            backends: vec![LspBackend {
                language: "rust".into(),
                backend: "rust-analyzer".into(),
                workspace_root: None,
            }],
            strategy: InitStrategy::Lazy,
            compression: false,
        });
        pool.insert_handle("rust", handle);

        let (uri, _path) = temp_file("fn main() {}");
        let result = pool.rename(&uri, "rust", 0, 3, "new_name").await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(parsed["changes"].is_object());
    }

    #[tokio::test]
    async fn test_mock_formatting() {
        let resp = LspMessage::Response {
            id: 1,
            result: Some(serde_json::json!([
                {
                    "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 12 } },
                    "newText": "fn main() {}\n",
                }
            ])),
            error: None,
        };

        let handle = mock_handle(vec![resp], false);
        let mut pool = LspPool::new(LspConfig {
            backends: vec![LspBackend {
                language: "rust".into(),
                backend: "rust-analyzer".into(),
                workspace_root: None,
            }],
            strategy: InitStrategy::Lazy,
            compression: false,
        });
        pool.insert_handle("rust", handle);

        let (uri, _path) = temp_file("fn main(){}");
        let result = pool.formatting(&uri, "rust", None).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed[0]["newText"], "fn main() {}\n");
    }

    #[tokio::test]
    async fn test_mock_send_raw() {
        let resp = LspMessage::Response {
            id: 1,
            result: Some(serde_json::json!({ "signatures": [{ "label": "fn main()" }] })),
            error: None,
        };

        let handle = mock_handle(vec![resp], false);
        let mut pool = LspPool::new(LspConfig {
            backends: vec![LspBackend {
                language: "rust".into(),
                backend: "rust-analyzer".into(),
                workspace_root: None,
            }],
            strategy: InitStrategy::Lazy,
            compression: false,
        });
        pool.insert_handle("rust", handle);

        let result = pool
            .send_raw(
                "rust",
                "textDocument/signatureHelp",
                serde_json::json!({
                    "textDocument": { "uri": "file:///test.rs" },
                    "position": { "line": 0, "character": 5 },
                }),
            )
            .await
            .unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["signatures"][0]["label"], "fn main()");
    }

    #[tokio::test]
    async fn test_mock_notify_change() {
        let handle = mock_handle(vec![], false);
        let mut pool = LspPool::new(LspConfig {
            backends: vec![LspBackend {
                language: "rust".into(),
                backend: "rust-analyzer".into(),
                workspace_root: None,
            }],
            strategy: InitStrategy::Lazy,
            compression: false,
        });
        pool.insert_handle("rust", handle);

        pool.notify_change("file:///test.rs", "rust", "new content")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_mock_notify_close() {
        let handle = mock_handle(vec![], false);
        let mut pool = LspPool::new(LspConfig {
            backends: vec![LspBackend {
                language: "rust".into(),
                backend: "rust-analyzer".into(),
                workspace_root: None,
            }],
            strategy: InitStrategy::Lazy,
            compression: false,
        });
        pool.insert_handle("rust", handle);

        pool.notify_change("file:///test.rs", "rust", "content")
            .await
            .unwrap();
        pool.notify_close("file:///test.rs", "rust").await.unwrap();
    }

    #[tokio::test]
    async fn test_mock_notify_save() {
        let handle = mock_handle(vec![], false);
        let mut pool = LspPool::new(LspConfig {
            backends: vec![LspBackend {
                language: "rust".into(),
                backend: "rust-analyzer".into(),
                workspace_root: None,
            }],
            strategy: InitStrategy::Lazy,
            compression: false,
        });
        pool.insert_handle("rust", handle);

        pool.notify_save("file:///test.rs", "rust").await.unwrap();
    }

    #[tokio::test]
    async fn test_mock_multi_language() {
        // Two handles for different languages
        let diag_rust = LspMessage::Notification {
            method: "textDocument/publishDiagnostics".into(),
            params: serde_json::json!({
                "uri": "file:///a.rs",
                "diagnostics": [{
                    "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 1, "character": 0 } },
                    "severity": 1,
                    "message": "rust diag",
                }],
            }),
        };
        let handle_rust = mock_handle(vec![diag_rust], false);

        let diag_py = LspMessage::Notification {
            method: "textDocument/publishDiagnostics".into(),
            params: serde_json::json!({
                "uri": "file:///b.py",
                "diagnostics": [{
                    "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 1, "character": 0 } },
                    "severity": 2,
                    "message": "python diag",
                }],
            }),
        };
        let handle_py = mock_handle(vec![diag_py], false);

        let mut pool = LspPool::new(LspConfig {
            backends: vec![
                LspBackend {
                    language: "rust".into(),
                    backend: "rust-analyzer".into(),
                    workspace_root: None,
                },
                LspBackend {
                    language: "python".into(),
                    backend: "basedpyright".into(),
                    workspace_root: None,
                },
            ],
            strategy: InitStrategy::Lazy,
            compression: false,
        });
        pool.insert_handle("rust", handle_rust);
        pool.insert_handle("python", handle_py);

        let (uri_a, _) = temp_file("fn main() {}");
        let (uri_b, _) = temp_file("x = 1");

        let result_a = pool.get_diagnostics(&uri_a, "rust").await.unwrap();
        let parsed_a: serde_json::Value = serde_json::from_str(&result_a).unwrap();
        assert_eq!(parsed_a["diagnostics"][0]["message"], "rust diag");

        let result_b = pool.get_diagnostics(&uri_b, "python").await.unwrap();
        let parsed_b: serde_json::Value = serde_json::from_str(&result_b).unwrap();
        assert_eq!(parsed_b["diagnostics"][0]["message"], "python diag");
    }
}
