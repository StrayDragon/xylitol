# c100-improve-test-harness Design

## Decisions

### 1. Unified timeout helper (`with_test_timeout`)

Generic async wrapper using `tokio::time::timeout` with configurable seconds.
Centralizes timeout logic so individual tests don't reimplement it.
Default timeout: 10 seconds (matches r5 spec).

### 2. MockToolContext workspace_root

Add `workspace_root: PathBuf` field initialized from `CARGO_MANIFEST_DIR`.
Provide `mock_context_with_root()` for custom root paths in tests.
No trait changes needed — field is accessed internally by mock implementations.

### 3. Incremental adoption

Wrap only known-hanging tests initially. Future tests can adopt the helper organically.
No blanket `#[timeout]` proc-macro — keeps dependency minimal.
