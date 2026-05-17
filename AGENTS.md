<!-- LLMANSPEC:START -->
# LLMAN 规范驱动开发

本项目使用 llman SDD。阅读 `llmanspec/config.yaml` 了解项目上下文与规则。

使用 `/llman-sdd-onboard` 开始，然后使用 `/llman-sdd-*` 技能进行工作流。

保留此托管块，便于 `llman sdd update` 刷新。
<!-- LLMANSPEC:END -->

# Repository Guidelines

## Project Structure & Module Organization

- Source lives in `src/`. As the project grows, keep the domain layering:
  - `src/agent/`: agent loop, tools, config, prompts
  - `src/infra/`: hooks, security, skills, session, planning
  - `src/interface/`: CLI/TUI/RPC and user-facing output
- Specs and workflow artifacts live in `llmanspec/`. General docs live in `docs/`.
- Build output is generated under `target/` (do not commit).

## Build, Test, and Development Commands

This repo uses `just` as the command runner:

- `just setup`: install `prek` git hooks (pre-commit/commit-msg/pre-push).
- `just fmt`: run `cargo fmt` (writes formatting).
- `just lint`: run `cargo clippy` (hooks may run with `-D warnings`).
- `just test`: run `cargo test`.
- `just qa`: `fmt-check + lint + test + doc-check`, then `prek run --all-files`.
- `just doc`: build docs with all features and open them.

## Coding Style & Naming Conventions

- Rust: Edition 2024 (`rust-toolchain.toml` pins stable + `rustfmt`/`clippy`).
- Formatting: `cargo fmt` (see `rustfmt.toml`: 4 spaces, `max_width = 100`).
- Errors: avoid `.unwrap()`/`.expect()` outside tests; prefer `thiserror` for library
  error enums and `anyhow` at application boundaries.
- Logging: use `tracing` (structured logs), not `log`.

## Testing Guidelines

- Default runner is `cargo test` (via `just test` / `just qa`).
- Naming: use `test_<feature>_<scenario>` for new tests.
- Async tests: wrap time-sensitive cases with `tokio::time::timeout(...)`.

## Commit & Pull Request Guidelines

- Commit messages are validated by `prek` (commit-msg hook). Use Conventional Commits:
  `type(scope)!: description` (keep the header ≤ 72 chars).
  Types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`,
  `chore`, `revert`.
- PRs: include a clear “what/why”, link relevant issues/specs, and ensure `just qa`
  passes locally. Add screenshots for UI changes (TUI/review modes) when applicable.

## Spec-Driven Development (llman SDD)

- When working on planned changes, follow llman SDD: specs/changes live under
  `llmanspec/`. Keep proposals and dependencies up to date (see `llmanspec/config.yaml`).
