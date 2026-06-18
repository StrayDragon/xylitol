# Handoff — pi-mono core alignment COMPLETE (2026-06-18)

## Context

All 11 planned changes (c30–c80) aligning `xylitol` (Rust) with `pi-mono` (TypeScript) coding agent core architecture are **COMPLETE**.

**Reference**: `/home/l8ng/Projects/__straydragon__/pi-mono/packages/coding-agent/src/core/`

## Completed (batches 1–6, all 11/11)

### Batch 1 — 4 independent changes

| Change | Module | What | Tests |
|--------|--------|------|-------|
| **c40** EventBus | `src/infra/event/mod.rs` | Channel-based EventBus: `emit`/`on`/`clear` with `tokio::spawn` handler isolation. | 4 |
| **c55** TrustManager | `src/infra/trust/mod.rs` | Trust store `~/.xylitol/trust.json` with atomic write, parent inheritance. | 8 |
| **c45** Skills | `src/infra/skills/loader.rs` | SKILL.md discovery, YAML frontmatter, name validation, ignore file support. | 7 |
| **c30** ModelRegistry | `src/agent/auth_storage.rs` + registry | AuthStorage, `ProviderApi` enum, `ProviderConfig.api/headers`, `filter_models`. | 6+3 |

### Batch 2 — 2 changes (both depend on c30)

| Change | Module | What | Tests |
|--------|--------|------|-------|
| **c35** SettingsManager | `src/infra/settings/` | 30-field `Settings` (camelCase), `deep_merge` 3-tier, retry-locked file storage. | 12 |
| **c50** Compaction | `src/infra/session/compaction.rs` | Token calculation, threshold check, `should_compact`, `XyUsage`. | 5 |

### Batch 3 — 2 changes (parallel, c60 depends on c30/c35/c40/c45/c55, c65 depends on c50)

| Change | Module | What | Tests |
|--------|--------|------|-------|
| **c60** ResourceLoader | `src/infra/resource/loader.rs` | `DefaultResourceLoader`: AGENTS.md/CLAUDE.md ancestor walking, SYSTEM.md/APPEND_SYSTEM.md, themes, diagnostics, `reload()`. | 19 lib |
| **c65** SessionManagerTree | `src/infra/session/` | `LabelEntry`, `SessionInfoEntry`, `SessionTreeNode`, `get_tree()`, `navigate_tree()`, `switch_session()`, label bookmarks. | 9 BDD |

### Batch 4 — c70 depends on c40/c60

| Change | Module | What | Tests |
|--------|--------|------|-------|
| **c70** AgentExtensions | `src/agent/extensions/` | `ToolDefinition` trait, `Extension` trait, `ExtensionLoader`, `ExtensionContext`, `ExtensionEvent`, before/after tool hooks. | 8 lib + 4 BDD |

### Batch 5 — c75 depends on all prior

| Change | Module | What | Tests |
|--------|--------|------|-------|
| **c75** AgentSession | `src/agent/session.rs`, `queue.rs` | `navigate_tree()`, `switch_session()`, `register_skill_commands()`, `wrap_registered_tools()`, message queue steering/follow-up, auto-persistence, auto-compaction. | 9 BDD session |

### Batch 6 — c80 depends on c30/c40/c75

| Change | Module | What | Tests |
|--------|--------|------|-------|
| **c80** AgentLoop | `src/agent/loop.rs` | `AgentHooks` (before/after tool, transform context, steering/follow-up msg callbacks), `ToolExecutionMode::Sequential/Parallel`, `SteeringMode`, `FollowUpMode`, auto-retry. | 2 loop tests |

**Total**: 312 unit + 83 BDD = 395 tests PASS. `fmt` ✅, `clippy` ✅.

## Key Design Decisions

1. **No backward compatibility**: Old code preserved with TODO markers until c75/c80 rewrite callers, then deleted in one pass.
2. **Coexistence strategy**: New infra modules live alongside old code. c75/c80 do the full switchover.
3. **File locking**: Atomic temp-file + rename (no `proper-lockfile` in Rust).
4. **Settings camelCase**: Serde `#[serde(rename_all = "camelCase")]` matches pi wire format.
5. **Storage abstraction**: `SettingsStorage` trait with file and memory backends.
6. **Specs skipped on archive**: Deltas merged via `--skip-specs` (code+test evidence sufficient; specs live as standalone `.toon`).

## Archived Changes

All 11 changes archived in `llmanspec/changes/archive/2026-06-18-*`.

## Quick Reference

```bash
# Run all lib tests (312)
cargo test --lib

# Run all BDD tests (83)
cargo test --test bdd -- --test-threads=1

# Run QA
just qa

# Check dependency graph
llman sdd graph

# Validate all specs
llman sdd validate --specs --strict --no-interactive
```

## Architecture Overview

```
src/
├── agent/
│   ├── extensions/     # c70 — ToolDefinition, Extension, ExtensionLoader, hooks
│   ├── loop.rs         # c80 — AgentLoop with hooks, steering/follow-up, tool modes
│   ├── prompt.rs       # c60 — SystemPromptOpts, build_system_prompt_from_loader
│   ├── queue.rs        # c75 — MessageQueue for steer/followUp
│   ├── session.rs      # c75 — AgentSession: fork, navigate, skill cmds, auto-persist
│   └── ...
├── infra/
│   ├── event/mod.rs    # c40 — EventBus
│   ├── resource/       # c60 — DefaultResourceLoader, diagnostics, themes
│   ├── session/        # c50 + c65 — compaction, tree ops, labels, session_info
│   ├── settings/       # c35 — SettingsManager with deep_merge
│   ├── skills/         # c45 — SKILL.md loader
│   └── trust/          # c55 — TrustManager
└── ...
```
