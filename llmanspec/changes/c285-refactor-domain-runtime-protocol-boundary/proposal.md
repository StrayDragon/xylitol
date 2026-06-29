---
id: c285-refactor-domain-runtime-protocol-boundary
depends_on:
  - c260-refactor-domain-architecture
  - c279-relocate-executor-and-config-resolution
---

# c285-refactor-domain-runtime-protocol-boundary

## Why

After `c260`/`c276`–`c279`, `src/core/` has drifted back into a mixed-responsibility
layer. It currently contains:

- **Pure domain vocabulary**: `AgentMessage`, `ModelConfig`, `SessionEntry`, `XyChunk`,
  `SourceInfo`, `XyError`, …
- **Runtime boundary traits**: `XyModel`, `XyTool`, `SessionStore`, `BashExecutor`,
  `SecretResolver`, `ResourceLoader`, …
- **Small mechanisms**: `parse_bang_prefix`, `session_export` rendering/parsing,
  `xml_escape`.

This violates the original intent that `core` should be a zero-dependency vocabulary
layer. The boundary traits deserve their own module, and the mechanisms belong in
`agent/` (orchestration) rather than in shared vocabulary.

This change performs a final structural cleanup:

1. Rename `core/` → `domain/` so its responsibility is unambiguous.
2. Extract the boundary traits into a new top-level `runtime_protocol/` module, organized by
   concern (`model`, `tool`, `session`, `event`, `sandbox`, `bash`, `secret`,
   `resource`, `trust`, `export`).
3. Move misplaced mechanisms back to their natural owners.
4. Introduce an async `ExportIo` port for session export/import file I/O so the
   agent layer stays free of direct filesystem side effects and future backends
   (gist, S3, clipboard) can be added without touching `agent/`.

There are **zero runtime behavior changes**.

## What Changes

### P0 — Rename `core` to `domain`

- Rename `src/core/` directory to `src/domain/`.
- Update `src/lib.rs` module declaration.
- Globally replace `crate::core::` with `crate::domain::`.
- Update thin re-exports in `infra/session/types.rs`, `infra/source_info.rs`,
  `infra/config/types.rs`, `infra/settings/types.rs`, and `infra/event/lifecycle.rs`.

### P1 — Extract `runtime_protocol/` as the agent↔infra boundary layer

- Create `src/runtime_protocol/` with submodules:
  - `model`: `XyModel`, `XyStream`, `ModelBuilder`
  - `tool`: `XyTool`, `XyToolCtx`, `ToolExecutionMode`
  - `session`: `SessionStore`
  - `event`: `EventSink`, `LifecycleHandler`
  - `sandbox`: `SandboxEngine`, `SandboxVerdict`
  - `bash`: `BashExecutor`, `BashResult`
  - `secret`: `SecretResolver`
  - `resource`: `ResourceLoader`
  - `trust`: `TrustStore`
  - `export`: `ExportIo`
- Move trait definitions and their signature-only associated types from
  `domain::ports` (the former `core::ports`).
- Update all imports from `crate::core::ports::` / `crate::domain::ports::` to
  `crate::runtime_protocol::`.

### P2 — Relocate mechanisms out of the vocabulary layer

- Move `core::bash::parse_bang_prefix` to `agent::session::bang`.
- Move `core::session_export` render/parse helpers to `agent::session::export`;
  delete `core::session_export`.
- Move `xml_escape` from `source_info.rs` to `domain::text`; `source_info.rs`
  keeps only data types.
- Move `LifecycleHandler` type alias from `domain::lifecycle` to `runtime_protocol::event`;
  `domain::lifecycle` keeps only the `AgentLifecycleEvent` enum.

### P3 — Add async `ExportIo` port and `StdExportIo`

- Define `#[async_trait] trait ExportIo` with `write_text` and `read_bytes`.
- Implement `StdExportIo` under `infra::export` using `tokio::fs`.
- Update `AgentSession::export_to_html/jsonl` and `import_from_jsonl` to use the
  injected `Arc<dyn ExportIo>`.
- Inject `Arc::new(StdExportIo)` in the CLI, RPC, and server composition roots.

### P4 — Documentation and spec synchronization

- Update `AGENTS.md` project structure description.
- Update active specs that reference `core::ports` / `core::` to `runtime_protocol::` /
  `domain::`.
- Clean up any remaining `infra/*` thin re-exports that are no longer needed.

## Capabilities

- `architecture` (add): module map update for the `domain/` / `runtime_protocol/` split.
- `layer-architecture` (modify): update `la1`/`la2`/`la7`/`la9` to reference
  `domain/` and `runtime_protocol/`; add scenario.
- `agent-runtime` (modify): update `ar17`/`ar18` to reference
  `runtime_protocol::SessionStore` / `runtime_protocol::EventSink`; add `LifecycleHandler` placement
  requirement.
- `agent-session` (modify + add): update `as34` path; add requirements for
  `ExportIo`, `bang` module relocation, and `session::export` relocation.
- `session-persistence` (modify + add): update `sp1`/`sp2` path; add requirement
  for `StdExportIo`.
- `tool-system` (modify): update `t17` to reference `runtime_protocol::XyTool`.
- `provider-integration` (modify): update `pi2` to reference `runtime_protocol::XyModel`.
- `interactive-client` (add): composition root must construct and inject
  `StdExportIo`.
- `server-runtime` (add): composition root must construct and inject
  `StdExportIo`.

## Impact

- **Behavior**: zero runtime behavior change; pure structural refactor.
- **Compilation**: large-scale import path rewrite; the compiler will flag every
  missed reference.
- **Tests**: all lib tests, BDD scenarios, and `src/tests.rs::arch_guard` must
  remain green.
- **Documentation**: `AGENTS.md` and active specs are updated; archived specs
  remain frozen.
- **Risk**: medium. The work is mechanical but touches many files; `ExportIo`
  injection changes `AgentSession::new` / `Agent::with_ports` signatures.
