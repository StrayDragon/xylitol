<!-- LLMANSPEC:START -->
# LLMAN 规范驱动开发

本项目使用 llman SDD。阅读 `llmanspec/config.yaml` 了解项目上下文与规则。

使用 `/llman-sdd-onboard` 开始，然后使用 `/llman-sdd-*` 技能进行工作流。

保留此托管块，便于 `llman sdd update` 刷新。
<!-- LLMANSPEC:END -->

# Repository Guidelines

## Project Structure & Module Organization

`xylitol` is a Rust 2024 CLI/agent toolkit structured as a thin orchestration core (`agent/`) over a large runtime domain (`infra/`), with swappable application surfaces (`app/`) speaking a single wire vocabulary (`protocol/`), plus an optional always-on server mode (`app/server/`). See `llmanspec/changes/archive/2026-06-26-c260-refactor-domain-architecture/design.md` for the layering invariants (HC-1…HC-6) enforced by `src/tests.rs::arch_guard`.

Source under `src/`:
- `domain/` — pure domain vocabulary + errors + serde types. Zero crate-internal deps.
- `runtime_protocol/` — agent↔infra boundary traits (`XyModel`, `XyTool`, `SessionStore`, `EventSink`, `BashExecutor`, `ExportIo`, …) + signature-only types. Depends only on `domain/`.
- `infra/` — **runtime domain**: `provider/` (LLM adapters, impl `XyModel`), `tools/` (built-in tool impls), `session/`, `sandbox/`, `process/`, `config/` (incl. `value.rs` secret resolution), `event/`, `hooks/`, `mcp/`, `skills/`, `resource/`, `trust/`, `git/`, `clipboard/`, `image/`, `tool_downloader/`, `export/` (`StdExportIo`).
- `agent/` — **thin orchestration**: `runtime/` (ReAct loop `react.rs`, `event.rs`, `hooks.rs`, queue/retry/stdout_guard), `facade.rs` (single public entry for interactive layers), `session/`, `model/` (registry + manager), `tools/` (`ToolRegistry` only — impls live in infra), `compaction/`, `prompt/` (system/commands/templates/skills).
- `protocol/` — client↔core wire vocabulary SSOT (`Command`/`Event` enums + transport helpers), transport-agnostic.
- `app/` — **application surfaces** (clients and entry points): `cli/` (incl. `provider_guidance.rs` presentation text), `print.rs`, `rpc.rs` (stdio transport over `protocol/`), `composition.rs` (shared Agent construction), `driver.rs` (`Driver` trait + `InProcessDriver`), `tui/` (feature-gated terminal UI incl. `diff_review/`), `gui.rs` (future GUI placeholder), and `server/` (feature-gated always-on HTTP/WebSocket server). `app/` modules depend only on `protocol/` + a `Driver`; only the composition roots import `agent`/`infra` together.

Integration and behavior tests live in `tests/`, with BDD feature files in `tests/features/`, shared harness code in `tests/support/`, and snapshot fixtures in `tests/support/snapshots/`. Architecture-layer guards live in `src/tests.rs::arch_guard`. Example config and schema files are in `configs/`; assets in `docs/assets/`; active and archived SDD specs are under `llmanspec/`.

## Build, Test, and Development Commands

- `just setup`: install `prek` hooks.
- `just fmt`: run `cargo fmt`.
- `just lint`: run `cargo clippy`.
- `just test`: run `cargo nextest run --profile ci` when available, otherwise `cargo test`.
- `just qa` or `just ci`: run format check, Clippy, tests, docs, and all `prek` hooks.
- `cargo run -- --help`: run the CLI locally and inspect available commands.
- `cargo doc --no-deps --all-features`: verify API docs build.

## Coding Style & Naming Conventions

Follow `rustfmt.toml`: Rust 2024 edition, 4-space indentation, Unix newlines, max width 100, reordered imports/modules, field init shorthand, and `?` shorthand. TOML/YAML files use 2-space indentation per `.editorconfig`. Prefer clear module boundaries that match the existing `agent`, `infra`, and `app` layers. Use snake_case for Rust modules, files, functions, and variables; use PascalCase for types and traits.

## Testing Guidelines

Use `cargo test` or `just test` for the full suite. BDD scenarios are defined in `tests/features/*.feature` and implemented in `tests/bdd.rs` with `rstest-bdd`; run targeted BDD tests with `cargo test bdd -- --test-threads=1` when ordering or shared state matters. Snapshot tests use `insta`; review snapshot changes before accepting them. Regression references belong in `tests/regression/` using `{issue_number}-{short-description}.rs`.

## Commit & Pull Request Guidelines

History and hooks expect Conventional Commits, for example `feat(cli): ...`, `fix(agent): ...`, `refactor(config): ...`, `docs: ...`, or `chore: ...`. Before opening a PR, run `just qa`. PR descriptions should summarize behavior changes, list test coverage, link related issues or `llmanspec/changes/...` items, and include screenshots or terminal output for CLI-visible changes.

## Provider Support Scope (Pre-1.0.0)

Only two provider APIs are supported:
- **OpenAI-compatible** (OpenAI Chat Completions format)
- **Anthropic** (Anthropic Messages API)

Other providers (Google, DeepSeek, NVIDIA, Groq, Mistral, OpenRouter, etc.), OAuth
credential storage, and provider-specific attribution headers are **not supported
until after 1.0.0**. Custom user-defined providers are accepted only if they
speak OpenAI-compatible or Anthropic-compatible APIs.

Code changes that add provider-specific logic for unsupported providers should
be rejected during review.

## Agent-Specific Instructions

For new feature iterations or refactors, do not add backwards-compatibility shims unless explicitly requested; update old call sites and formats to the new approach in one pass. Keep llman SDD artifacts synchronized when implementing planned changes.
