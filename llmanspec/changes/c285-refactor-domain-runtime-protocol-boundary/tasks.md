# c285-refactor-domain-runtime-protocol-boundary — Tasks

Each task targets ≤2 hours of focused work. Validate after every task before
moving on.

## T0 — Bootstrap change artifacts

- [x] Create `llmanspec/changes/c285-refactor-domain-runtime-protocol-boundary/`.
- [x] Write `proposal.md`, `design.md`, and delta specs.
- [x] Run `llman sdd validate c285-refactor-domain-runtime-protocol-boundary --strict --no-interactive`.

**Validation**: `llman sdd validate c285-refactor-domain-runtime-protocol-boundary --strict --no-interactive` passes.

## T1 — Rename `core/` to `domain/`

- [x] `git mv src/core src/domain`.
- [x] Update `src/lib.rs`: `pub mod core;` → `pub mod domain;`.
- [x] Replace `crate::core::` with `crate::domain::` across `src/`.
- [x] Update `infra/session/types.rs`, `infra/source_info.rs`, `infra/event/lifecycle.rs`,
      `infra/config/types.rs`, `infra/settings/types.rs` re-exports to point to `domain`.
- [x] Update comments/doc strings that mention `core/`.

**Validation**: `cargo build --lib` passes.

## T2 — Extract `runtime_protocol/` model, tool, session, event

- [x] Create `src/runtime_protocol/mod.rs`.
- [x] Create `src/runtime_protocol/model.rs` with `XyModel`, `XyStream`, `ModelBuilder`.
- [x] Create `src/runtime_protocol/tool.rs` with `XyTool`, `XyToolCtx`, `ToolExecutionMode`.
- [x] Create `src/runtime_protocol/session.rs` with `SessionStore`.
- [x] Create `src/runtime_protocol/event.rs` with `EventSink`; move `LifecycleHandler` here.
- [x] Remove the corresponding sections from `domain::ports`.
- [x] Update imports: `crate::core::ports::` / `crate::domain::ports::` → `crate::runtime_protocol::`.

**Validation**: `cargo build --lib` passes.

## T3 — Extract remaining `runtime_protocol/` submodules

- [x] Create `src/runtime_protocol/sandbox.rs` with `SandboxEngine`, `SandboxVerdict`.
- [x] Create `src/runtime_protocol/bash.rs` with `BashExecutor`, `BashResult`.
- [x] Create `src/runtime_protocol/secret.rs` with `SecretResolver`.
- [x] Create `src/runtime_protocol/resource.rs` with `ResourceLoader`.
- [x] Create `src/runtime_protocol/trust.rs` with `TrustStore`.
- [x] Remove the rest of `domain::ports` and delete `src/domain/ports.rs`.
- [x] Update imports across `agent/` and `infra/`.

**Validation**: `cargo build --lib` passes.

## T4 — Add async `ExportIo` port

- [x] Create `src/runtime_protocol/export.rs` with async `ExportIo` trait (`write_text`, `read_bytes`).
- [x] Create `src/infra/export.rs` with `StdExportIo` using `tokio::fs`.
- [x] Update `AgentSession` to hold `export_io: Arc<dyn ExportIo>`.
- [x] Update `AgentSession::export_to_html/jsonl` and `import_from_jsonl` to use `ExportIo`.
- [x] Update `Agent::with_ports` / `AgentSession::new` signatures.
- [x] Inject `StdExportIo` in `interactive/cli`, `interactive/rpc`, and `server/runtime`.

**Validation**: `cargo build --lib` and `cargo test --lib` pass.

## T5 — Relocate mechanisms out of `domain/`

- [x] Move `parse_bang_prefix` to `agent::session::bang` and update call sites.
- [x] Move `core::session_export` render/parse helpers to `agent::session::export`;
      replace `write_to` with `ExportIo::write_text`.
- [x] Move `xml_escape` to `domain::text` and update call sites.
- [x] Move `LifecycleHandler` alias to `runtime_protocol::event`; keep `AgentLifecycleEvent` in
      `domain::lifecycle`.
- [x] Delete `src/domain/bash.rs` and `src/domain/session_export.rs`.

**Validation**: `cargo build --lib` and `cargo test --lib` pass.

## T6 — Update tests, specs, and docs

- [x] Update any test code that imports `crate::core::` or `crate::domain::ports::`.
- [x] Update `src/tests.rs::arch_guard` comments if they reference `core/`.
- [x] Update active specs that mention `core::ports` / `core::` to `runtime_protocol::` / `domain::`.
- [x] Update `AGENTS.md` project structure description.
- [x] Clean up stale `infra/*` re-exports that are no longer needed.

**Validation**:

- `cargo test --lib`
- `cargo test --lib arch_guard`
- `llman sdd validate c285-refactor-domain-runtime-protocol-boundary --strict --no-interactive`

## T7 — Full QA and BDD

- [x] `cargo fmt --check`
- [x] `cargo clippy --lib`
- [x] `cargo test --lib`
- [x] `cargo test --test bdd -- --test-threads=1`
- [x] `llman sdd validate --all`

**Done criteria**: all checks green.
