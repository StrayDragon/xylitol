---
depends_on: [c30-add-print-mode]
---

# c03-update-cli-mode-dispatch

## Why

Current CLI uses an explicit `--mode` flag with a `RunMode` enum (Print, Interactive, ACP) defaulting to "print". This forces users to learn a mode taxonomy upfront. A more natural UX derives mode from user intent:

- `xylitol "do something"` → print/stdio (prompt present)
- `xylitol` → interactive TUI (no prompt)
- `xylitol --acp` → ACP mode (explicit opt-in for IDE integration)

Additionally, there is no way to inspect available models from config (`--list-models`), and the fake provider has no CLI entry point despite being fully implemented behind `dev-fake-provider`.

`--mode` was never in a released version, so removing it is safe.

## What Changes

1. **Remove `--mode` flag and `RunMode` enum** — mode is now auto-detected from positional args and flags.
2. **Add `--acp` boolean flag** (feature-gated `infra-acp`) as a direct opt-in for ACP protocol.
3. **Add `--list-models` early-exit flag** — prints config models + `__fake__` (when `dev-fake-provider` enabled) and exits.
4. **Add `__fake__` sentinel model** — `--model __fake__` activates the fake provider from CLI, bypassing config lookup and API key resolution.

## Capabilities

- auto-mode-detect: CLI mode derived from positional args + flags instead of explicit enum
- list-models: `--list-models` flag prints available model entries from config
- fake-model-cli: `--model __fake__` activates the fake provider from CLI

## Impact

- Breaking: `--mode` flag removed (never released, safe)
- Primary: `src/interface/cli/mod.rs`, `src/agent/model.rs`
- Spec: `llmanspec/specs/cli-entry/spec.md`
- No new dependencies
