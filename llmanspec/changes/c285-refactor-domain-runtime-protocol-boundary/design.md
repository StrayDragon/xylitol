# c285-refactor-domain-runtime-protocol-boundary — Design

## 0. Problem statement

`src/core/` currently hosts three different kinds of code:

1. **Domain vocabulary** — `AgentMessage`, `ModelConfig`, `SessionEntry`, `XyChunk`, …
2. **Runtime boundary traits** — `XyModel`, `XyTool`, `SessionStore`, `BashExecutor`, …
3. **Small mechanisms** — `parse_bang_prefix`, `session_export` rendering, `xml_escape`.

This makes the layer’s contract unclear and makes it harder to explain where a new
type should go. The goal is to split these into three clearly named homes:

- `domain/` — pure data + errors, zero crate-internal dependencies.
- `runtime_protocol/` — trait contracts between `agent/` and `infra/`.
- `agent/` / `infra/` — the actual owners of mechanisms and implementations.

## 1. New module map

```text
src/
├── domain/            # 纯领域词汇 + 错误
│   ├── message.rs
│   ├── model.rs
│   ├── types.rs
│   ├── error.rs
│   ├── session_types.rs
│   ├── resource_types.rs
│   ├── source_info.rs
│   ├── lifecycle.rs   # AgentLifecycleEvent 枚举（去掉 Handler 别名）
│   ├── compaction_config.rs
│   └── text.rs        # xml_escape 等纯文本小工具
│
├── runtime_protocol/             # agent↔infra 边界 trait + 签名类型
│   ├── mod.rs
│   ├── model.rs       # XyModel, XyStream, ModelBuilder
│   ├── tool.rs        # XyTool, XyToolCtx, ToolExecutionMode
│   ├── session.rs     # SessionStore
│   ├── event.rs       # EventSink, LifecycleHandler
│   ├── sandbox.rs     # SandboxEngine, SandboxVerdict
│   ├── bash.rs        # BashExecutor, BashResult
│   ├── secret.rs      # SecretResolver
│   ├── resource.rs    # ResourceLoader
│   ├── trust.rs       # TrustStore
│   └── export.rs      # ExportIo
│
├── agent/             # 编排：ReAct, session, prompt, registries
│   └── session/
│       ├── bang.rs    # parse_bang_prefix
│       └── export.rs  # render_html, render_jsonl, parse_jsonl
│
├── infra/             # 运行时实现：impl runtime_protocol
│   ├── export.rs      # StdExportIo
│   └── ...
│
├── protocol/          # client↔core 线协议（不变）
├── interactive/
└── server/
```

## 2. Dependency direction

```text
interactive → protocol
                 ↓
server → agent → runtime_protocol → domain
          ↑       ↑
        infra ────┘
```

Hard invariants (same as HC-1):

- `domain` has zero crate-internal dependencies.
- `runtime_protocol` depends only on `domain`.
- `agent` depends on `domain` + `runtime_protocol`; it never imports concrete `infra` types.
- `infra` depends on `domain` + `runtime_protocol`; it never imports `agent`.
- `interactive` depends only on `protocol` (+ the in-process composition root).

## 3. Decision matrix：什么放哪里

| 判定条件 | 放 `domain/` | 放 `runtime_protocol/` | 放 `agent/` / `infra/` |
|---|---|---|---|
| 跨层共享的纯数据/serde 类型 | ✅ | ❌ | ❌ |
| trait 契约或签名专属类型 | ❌ | ✅ | ❌ |
| 只在单一使用处有语义 | ❌ | ❌ | ✅ |
| 纯函数工具（无业务状态） | ✅（`domain::text`） | ❌ | ❌ |
| I/O 或具体运行时 | ❌ | ❌ | ✅ |

Concrete examples:

- `AgentMessage`, `SessionEntry`, `ModelConfig`, `XyChunk`, `SourceInfo`, `XyError`
  → `domain/`
- `XyModel`, `XyTool`, `SessionStore`, `EventSink`, `BashExecutor`, `ExportIo`,
  `XyToolCtx`, `BashResult`, `SandboxVerdict`, `ModelBuilder`, `LifecycleHandler`
  → `runtime_protocol/`
- `parse_bang_prefix`, session export render/parse → `agent/`
- `InfraBashExecutor`, `StdExportIo`, `OpenAIProvider`, `SessionManager` → `infra/`

## 4. `ExportIo` port design

We deliberately make the port **async from day one** because:

- The project already uses `tokio` everywhere.
- The most likely second implementation (GitHub gist, S3, clipboard) is a network
  or external I/O backend that wants async.
- Changing a sync port to async later is a breaking signature change across the
  facade and all composition roots; doing it now avoids that cost.

```rust
// src/runtime_protocol/export.rs
#[async_trait]
pub trait ExportIo: Send + Sync {
    async fn write_text(&self, path: &Path, content: &str) -> Result<(), String>;
    async fn read_bytes(&self, path: &Path) -> Result<Vec<u8>, String>;
}
```

`infra::export::StdExportIo` implements it with `tokio::fs::write` and
`tokio::fs::read`.

`AgentSession` holds `export_io: Arc<dyn ExportIo>` and uses it in
`export_to_html`, `export_to_jsonl`, and `import_from_jsonl`.

The CLI, RPC, and server composition roots inject `Arc::new(StdExportIo)`.

## 5. Migration mechanics

### 5.1 `core/` → `domain/`

- `git mv src/core src/domain`
- Update `src/lib.rs`: `pub mod core;` → `pub mod domain;`
- Global replace `crate::core::` → `crate::domain::`
- Update active specs and `AGENTS.md`

No backward-compatibility shim (`pub use domain as core`) per project rule.

### 5.2 `domain::ports` → `runtime_protocol/`

- Create `src/runtime_protocol/`.
- Split `domain::ports.rs` into per-concern submodules.
- Move signature-only associated types with their traits:
  - `XyToolCtx`, `ToolExecutionMode` → `runtime_protocol::tool`
  - `BashResult` → `runtime_protocol::bash`
  - `SandboxVerdict` → `runtime_protocol::sandbox`
  - `LifecycleHandler` → `runtime_protocol::event`
- Update all imports: `crate::core::ports::` / `crate::domain::ports::` →
  `crate::runtime_protocol::`.

### 5.3 Mechanisms relocation

- `parse_bang_prefix` → `agent::session::bang::parse_bang_prefix`
- `core::session_export::{render_html, render_jsonl, parse_jsonl}` →
  `agent::session::export`
- `core::session_export::write_to` → replaced by `ExportIo::write_text`
- `core::session_export::share_guidance_message` → stays with
  `agent::session::export` as command-response text (interactive just prints it)
- `xml_escape` → `domain::text::xml_escape`
- `LifecycleHandler` → `runtime_protocol::event::LifecycleHandler`

## 6. Risks and mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| Large import-path blast radius | High churn, easy to miss references | `cargo build` after each task; grep for `crate::core::` and `runtime_protocol::` before final validation |
| `ExportIo` async changes `AgentSession::new` / `Agent::with_ports` signatures | Composition roots and tests need updates | Update cli/rpc/server and all tests in the same change; no shim |
| Active specs mention `core::ports` | Spec validation may fail | Update specs in P4; run `llman sdd validate --all` |
| `infra/*` thin re-exports become stale | Compilation errors or misleading imports | Audit `infra/session/types.rs`, `infra/source_info.rs`, `infra/event/lifecycle.rs`, `infra/config/types.rs`, `infra/settings/types.rs` |

## 7. Validation strategy

1. `cargo fmt --check`
2. `cargo clippy --lib`
3. `cargo test --lib`
4. `cargo test --test bdd -- --test-threads=1`
5. `cargo test --lib arch_guard`
6. `llman sdd validate c285-refactor-domain-runtime-protocol-boundary --strict --no-interactive`
7. `llman sdd validate --all`

All checks must be green before archiving.
