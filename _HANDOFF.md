# Handoff — pi-mono core alignment (2026-06-18)

## Context

Systematically aligning `xylitol` (Rust) with `pi-mono` (TypeScript) coding agent core architecture
through 11 planned changes (c30–c80).

**Reference**: `/home/l8ng/Projects/__straydragon__/pi-mono/packages/coding-agent/src/core/`

## Completed (batches 1–3)

### Batch 1 — 4 independent changes

| Change | Module | What | Tests |
|--------|--------|------|-------|
| **c40** EventBus | `src/infra/event/mod.rs` | Channel-based EventBus: `emit(channel, data)` / `on(channel, handler)` / `clear()` with `tokio::spawn` handler isolation, standard channel names. Old `src/agent/event.rs` preserved with TODO(c75). | 4 |
| **c55** TrustManager | `src/infra/trust/mod.rs` | Trust store `~/.xylitol/trust.json` with atomic write (temp+rename), parent inheritance, session-only trust options, trust-requiring resource detection. | 8 |
| **c45** Skills | `src/infra/skills/loader.rs` | SKILL.md discovery, YAML frontmatter parsing, name validation (Agent Skills spec), ignore file support. Old code moved to `manager.rs` for c75 cleanup. | 7 |
| **c30** ModelRegistry | `src/agent/auth_storage.rs` + registry/resolver enhancement | AuthStorage OAuth persistence, `ProviderApi` enum (OpenAiCompatible/AnthropicMessages/OpenAiResponses), `ProviderConfig.api/headers` fields, `ProviderConfig::custom()` constructor, `filter_models(patterns)` with provider/* wildcard. | 6+3 |

### Batch 2 — 2 changes (both depend on c30)

| Change | Module | What | Tests |
|--------|--------|------|-------|
| **c35** SettingsManager | `src/infra/settings/` (types, storage, manager) | 30-field `Settings` struct (camelCase serde), `deep_merge` 3-tier (global<project<overrides), `FileSettingsStorage` with retry locking (10 attempts, 20ms delay), `InMemorySettingsStorage`, `SettingsManager` with accessors/mutators/reload/project_trust toggle. Lives alongside old `src/infra/config/`. | 12 |
| **c50** Compaction | `src/infra/session/compaction.rs` | `calculate_context_tokens(usage)` — total_tokens priority > sum fallback; `estimate_context_tokens(messages, last_usage)` — chars/4 heuristic; `should_compact(tokens, window, settings)` — threshold check. Added `XyUsage` struct. | 5 |

### Batch 3 — 2 changes (parallel, both ✅ ARCHIVED)

| Change | Module | What | Tests |
|--------|--------|------|-------|
| **c60** ResourceLoader | `src/infra/resource/loader.rs` | `DefaultResourceLoader` with eager load + `reload()`: AGENTS.md/CLAUDE.md discovery (ancestor walking + global), SYSTEM.md/APPEND_SYSTEM.md discovery, prompt templates (global<project), skills (SKILL.md frontmatter), themes (.json from dirs), `ResourceDiagnostic` error/warning collection, `PromptToolsOpts` + `build_system_prompt_from_loader()`. Old `resource.rs` deleted → module dir. | 19 lib |
| **c65** SessionManagerTree | `src/infra/session/manager.rs` + `types.rs` | `LabelEntry`/`SessionInfoEntry` types, `SessionTreeNode`, `get_tree()` tree traversal, `navigate_tree()` leaf switch, `switch_session()` file switch, `append_label_change()`/`get_label()` bookmark support, `append_session_info()`/`get_session_name()` metadata. BDD + label scenarios (set/clear). | 9 BDD (session) |

**Total**: 305 unit + 79 BDD = 384 tests PASS. `fmt` ✅, `clippy` ✅.

## Key Design Decisions

1. **No backward compatibility**: Old code preserved with TODO markers until c75/c80 rewrite callers, then deleted in one pass.
2. **Coexistence strategy**: New infra modules (`infra/settings/`, `infra/event/`, `infra/trust/`, `infra/skills/loader.rs`, `infra/resource/`) live alongside old code (`infra/config/`, `agent/event.rs`). Downstream changes (c75/c80) do the full switchover.
3. **File locking**: Uses atomic temp-file + rename instead of `proper-lockfile` (no Rust equivalent).
4. **Settings camelCase**: Serde `#[serde(rename_all = "camelCase")]` matches pi wire format.
5. **Storage abstraction**: `SettingsStorage` trait with `FileSettingsStorage` and `InMemorySettingsStorage` backends.

## Dependency Graph (remaining)

```
已完成 (8/11): c30 c35 c40 c45 c50 c55 c60 c65

批次 4:
  c70 AgentExtensions → depends on c40 c60

批次 5:
  c75 AgentSession → depends on c30 c35 c40 c50 c60 c65 c70

批次 6:
  c80 AgentLoop → depends on c30 c40 c75
```

## Remaining Work

### c70 — Agent Extensions (pi's `extensions/types.ts`)
- Extension loading from npm packages and local dirs
- Skill/prompt/theme/resource extension types
- Extension lifecycle (activate/deactivate)

### c75 — Agent Session (pi's `agent-session.ts`)
- **Biggest change**: Full rewrite of `AgentSession`
- Switch all call sites from old modules to new infra
- Wire EventBus, TrustManager, Skills, SettingsManager, ResourceLoader
- Delete all TODO(c75) old code

### c80 — Agent Loop (pi's `agent-loop.ts`)
- Final integration: wire everything into agent loop
- Model switching, compaction trigger check, extension hooks

## Files to Delete in c75/c80

These files exist alongside new modules and are scheduled for deletion:

- `src/agent/event.rs` → replaced by `src/infra/event/mod.rs`
- `src/infra/skills/manager.rs` → replaced by `src/infra/skills/loader.rs`
- Old `ModelRegistry::get_at()` and index-based code → replaced by `filter_models`
- `src/infra/config/types.rs` settings-related fields → replaced by `src/infra/settings/`

## Quick Reference

```bash
# Run all lib tests
cargo test --lib

# Run BDD tests (all)
cargo test --test bdd -- --test-threads=1

# Run QA
just qa

# Check dependency graph
llman sdd graph

# List remaining changes
llman sdd list
```

## Next Session

1. Start with `just test` to verify all 384 tests still pass
2. Implement **c70 AgentExtensions** — read `pi-mono/.../extensions/types.ts` for reference
3. Then **c75 AgentSession** — the big one
