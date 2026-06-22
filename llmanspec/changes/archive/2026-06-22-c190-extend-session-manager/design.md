# Design: c190-extend-session-manager

## Session Tree Structure

```
Root (header)
├── Entry-0 (user message)
├── Entry-1 (assistant message)
├── Entry-2 (tool result)
├── Entry-3 (user message) ← fork point
│   ├── Entry-4 (assistant, branch A) ← leaf
│   └── Entry-4b (assistant, branch B) ← leaf (from fork)
```

## In-Memory Mode

```rust
pub enum SessionBackend {
    Persisted { sessions_dir: PathBuf },
    InMemory { entries: Vec<SessionEntry> },
}
```
