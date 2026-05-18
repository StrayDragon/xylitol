# _HANDOFF.md — Project State

> Last updated: 2026-05-18

## Recently Completed

### c40-add-hooks — Hook Event System
- [x] HookEvent enum (15 event types), HookPhase (Pre/Post), HookAction (Allow/Block/Modify)
- [x] HookDispatcher with three-tier config merge (global > project > user)
- [x] Script execution via tokio::process (stdin JSON context → stdout control directive)
- [x] Event pattern matching: `phase.event.qualifier` (e.g. `pre.tool_call.bash`)
- [x] Integration: `AgentLoop` carries optional `HookDispatcher`, `run_print_mode` dispatches events
- [x] **Blocks**: c50-add-security, c75-add-diff-review

### c35-add-repeat-detection
- [x] N-gram repeat detection middleware for LLM output streams
- [x] RecoveryManager with sequential recovery chain
- [x] Config-gated (default disabled), always compiled

## Active Changes (in priority order — next to apply)

| ID | Name | Tasks | Priority Note |
|----|------|-------|--------------|
| c45 | add-lsp-layer | 0/9 | |
| c50 | add-security | 0/10 | Blocked by c40 hooks |
| c55 | add-planning-execution | 0/10 | |
| c60 | add-model-lock | 0/8 | |
| c65 | add-skills-mcp | 0/9 | |
| c70 | add-session-snapshot | 0/12 | |
| c75 | add-diff-review | 0/9 | Blocked by c40 hooks |
| c80 | add-tui | 0/13 | |
| c85 | add-dap-layer | 0/7 | |
| c87 | add-acp-mode | 0/8 | |
| c88 | add-test-infra | 0/13 | |

## Verification State

- `just fmt`: ✓
- `cargo clippy -- -D warnings`: ✓
- `cargo test --lib`: 139 passed
- `llman sdd validate --all`: ✓

## Key Architecture

- `src/agent/loop.rs` — AgentLoop (Runner + HookDispatcher + repeat detection)
- `src/agent/repeat.rs` — RepeatDetector, RepeatDetectorStream, RecoveryManager
- `src/agent/tools/` — Tool implementations + ToolRegistry
- `src/infra/hooks/` — HookDispatcher, HookEvent, script execution
- `src/infra/config/` — AppConfig, multi-layer loader, JSON Schema validation
- `src/interface/print.rs` — Print mode (stdout stream consumer + hook dispatch)
- `src/interface/cli/mod.rs` — CLI arg parsing + mode dispatch

## Next Steps

Apply next change: `llman sdd list` to confirm priority, then `/llman-sdd-apply <id>`.
