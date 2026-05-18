# Design — c03-update-cli-mode-dispatch

## Context

PRD section: CLI entry (`llmanspec/specs/cli-entry/spec.md`). This change restructures how the CLI determines which mode to run, moving from an explicit `--mode` enum to implicit auto-detection.

## Goals

- Natural UX: prompt → print, no prompt → TUI, `--acp` → ACP
- Diagnostic capability: `--list-models` for inspecting config
- Dev experience: `--model __fake__` for smoke testing without API keys
- Zero impact when features (`ui-tui`, `infra-acp`, `dev-fake-provider`) are disabled

## Non-Goals

- Customizing the fake provider scenario from CLI (future: `--fake-scenario` or config)
- Model discovery via API calls (config-only)
- Backward compatibility with `--mode` (never released)

## Decisions

### D1: Auto-detection via if-else chain

Dispatch order: `--list-models` → `--acp` → `prompt.is_some()` → `prompt.is_none()`.

`--list-models` loads config first (models come from config) then exits. `--acp` is checked before prompt — ACP protocol drives prompts over stdio, so CLI prompt is irrelevant.

### D2: `ModelKind::Fake` variant (feature-gated)

Instead of bypassing `ModelConfig` entirely, add `ModelKind::Fake` behind `dev-fake-provider`. This keeps the `build_model_config() → ModelConfig → build() → Arc<dyn Llm>` pipeline intact.

Default scenario: single `ScenarioStep::text("Hello from __fake__ provider")` for quick smoke testing.

### D3: `--list-models` output format

Plain text table with alias, provider, model name columns. Default model marked with `*`. `__fake__` appended conditionally.

## Risks

| Risk | Level | Mitigation |
|------|-------|------------|
| `--mode` removal | Low | Never released, no external users |
| `ModelKind::Fake` leaks into model module | Low | Feature-gated, zero impact on production builds |
| `--list-models` fails if config broken | Medium | Acceptable: broken config is a problem regardless |
| `__fake__` collision with user model name | Very Low | Double-underscore convention is unlikely; document sentinel |
