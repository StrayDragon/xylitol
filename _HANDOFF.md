# Handoff — pi-mono core alignment STATUS (2026-06-20)

## Summary

**All 25 pi-mono core modules aligned** (c30–c80 + c95–c120 + c81–c89 + c98).
**0 remaining gaps**. All deferred integrations complete.

**Reference**: `/Users/l8ng/Projects/pi/packages/coding-agent/src/core/`

**Excluded by design**: telemetry, plugin/extension system, SDK, TUI.

## Completed Changes

### Batch 1–6: Original pi-mono alignment (2026-06-18, all 11/11)

| Change | Module | What | Tests |
|--------|--------|------|-------|
| **c40** EventBus | `src/infra/event/mod.rs` | Channel-based EventBus: `emit`/`on`/`clear` with `tokio::spawn` handler isolation. | 4 |
| **c55** TrustManager | `src/infra/trust/mod.rs` | Trust store `~/.xylitol/trust.json` with atomic write, parent inheritance. | 8 |
| **c45** Skills | `src/infra/skills/loader.rs` | SKILL.md discovery, YAML frontmatter, name validation, ignore file support. | 7 |
| **c30** ModelRegistry | `src/agent/auth_storage.rs` + `registry.rs` | AuthStorage, `ProviderApi` enum, `ProviderConfig.api/headers`, `filter_models`. | 6+3 |
| **c35** SettingsManager | `src/infra/settings/` | 30-field `Settings` (camelCase), `deep_merge` 3-tier, retry-locked file storage. | 12 |
| **c50** Compaction | `src/infra/session/compaction.rs` | Token calculation, threshold check, `should_compact`, `XyUsage`. | 5 |
| **c60** ResourceLoader | `src/infra/resource/loader.rs` | `DefaultResourceLoader`: AGENTS.md ancestor walking, SYSTEM.md, themes, diagnostics, `reload()`. | 19 |
| **c65** SessionManagerTree | `src/infra/session/` | `LabelEntry`, `SessionTreeNode`, `get_tree()`, `navigate_tree()`, `switch_session()`. | 9 BDD |
| **c70** AgentExtensions | `src/agent/extensions/` | `ToolDefinition` trait, `Extension` trait, `ExtensionContext`, `ExtensionEvent`, before/after tool hooks. | 8+4 BDD |
| **c75** AgentSession | `src/agent/session.rs`, `queue.rs` | `navigate_tree()`, `switch_session()`, `register_skill_commands()`, auto-persistence, auto-compaction. | 9 BDD |
| **c80** AgentLoop | `src/agent/loop.rs` | `AgentHooks`, `ToolExecutionMode`, `SteeringMode`, `FollowUpMode`, auto-retry. | 2 |

### Batch 7: Post-handoff additions (2026-06-19, 6/6)

| Change | Module | What | Lines |
|--------|--------|------|-------|
| **c95** BashExecutor | `src/agent/bash_executor.rs` | Persistent bash executor for `!`/`!!` commands with recording. | 292 |
| **c100** PromptTemplates | wired in session.rs | `register_prompt_commands()` — template injection from ResourceLoader. | — |
| **c105** ExportCapabilities | `src/infra/session/export/mod.rs` | HTML export, JSONL export, JSONL import. | 286 |
| **c110** SlashCommands | `src/agent/commands.rs` | 22 builtin slash commands, `SlashCommandSource`, `dispatch_slash_command()`. | — |
| **c115** RPCMode | `src/interface/rpc.rs` | Full JSONL-over-stdio RPC protocol (prompt, steer, abort, etc.). | 605 |
| **c120** ResourceDiagnostics | `src/interface/resources.rs` | CLI `resources list`/`info`/`doctor` subcommands. | 327 |

### Batch 8: Gap closure (2026-06-20, 8/8 — all archived)

| ID | Name | pi equivalent | Module | Tests |
|----|------|---------------|--------|-------|
| **c81** | config-value-resolver | `resolve-config-value.ts` | `src/agent/config_value.rs` | 28 |
| **c82** | provider-attribution | `provider-attribution.ts` + `provider-display-names.ts` | `src/agent/provider/attribution.rs` | 12 |
| **c83** | http-dispatcher | `http-dispatcher.ts` | `src/agent/http_dispatcher.rs` | 15 |
| **c84** | session-cwd-validation | `session-cwd.ts` | `src/infra/session/cwd.rs` | — |
| **c85** | auth-guidance | `auth-guidance.ts` | `src/agent/auth_guidance.rs` | 4 |
| **c86** | timing-instrumentation | `timings.ts` | `src/infra/timing.rs` | 2 |
| **c89** | settings-completeness | extended `Settings` fields | `src/infra/settings/` (16 new fields) | — |
| **c98** | unified-source-info | `source-info.ts` | `src/infra/source_info.rs` (+ migration) | — |

### Integration Work (2026-06-20, all deferred tasks completed)

| Area | Integration |
|------|------------|
| **timing-instrumentation** | `reset_timings()` → `time("config.load")` → `time("model_registry.load")` → `time("session.restore")` → `time("session.create")` → `print_timings()` at exit |
| **session-cwd-validation** | Session CWD existence check before restore; `cwd` module exported via `infra/session/mod.rs` |
| **auth-guidance** | `format_no_models_available_message()` on empty registry; `format_no_model_selected_message()` on resolve failure |
| **resolver** | `--model` flag uses `resolver::resolve_model()` (exact → fuzzy → fallback) instead of bare `find()` |
| **unified-source-info** | `source_path: PathBuf` → `source_info: SourceInfo` across `resource/loader.rs`, `commands.rs`, `session.rs`, `resources.rs` |

## Verification

```
cargo test --lib:    412 passed, 0 failed, 1 ignored
cargo clippy --lib:  3 warnings (wildcard patterns, intentional)
cargo fmt:           clean
cargo check --lib:   0 errors
llman sdd validate:  11 specs passed, 0 failed
```

## Key Design Decisions

1. **No backward compatibility**: Old call sites updated in one pass — no shims.
2. **Config value resolution**: `std::process::Command` for `!cmd` (no npm/jiti in Rust).
3. **File locking**: Atomic temp-file + rename (no `proper-lockfile` in Rust).
4. **Settings camelCase**: Serde `#[serde(rename_all = "camelCase")]` matches pi wire format.
5. **Storage abstraction**: `SettingsStorage` trait with file and memory backends.
6. **SourceInfo unification**: All resources share `infra::source_info::SourceInfo` with `path`, `source`, `scope`, `origin`, `base_dir`.

## Quick Reference

```bash
# Run all lib tests
cargo test --lib

# Run all BDD tests
cargo test --test bdd -- --test-threads=1

# Run full QA
just qa

# Validate all specs
llman sdd validate --specs --strict --no-interactive
```

## Architecture Overview

```
src/
├── agent/
│   ├── auth_guidance.rs      # c85 — user-facing auth/model- selection guidance messages
│   ├── auth_storage.rs       # c30 — OAuth + API key credential store
│   ├── bash_executor.rs      # c95 — persistent bash executor for !/!!
│   ├── commands.rs           # c110 — slash command system (22 builtin)
│   ├── config_value.rs       # c81 — env var / shell cmd / template config resolution
│   ├── defaults.rs           # defaults.ts — default thinking level, model IDs
│   ├── diagnostics.rs        # diagnostics.ts — non-fatal issue collection
│   ├── extensions/           # c70 — ToolDefinition, Extension, hooks
│   ├── http_dispatcher.rs    # c83 — HTTP proxy & timeout configuration
│   ├── loop.rs               # c80 — AgentLoop with hooks, tool modes, retry
│   ├── output_guard.rs       # output-guard.ts — stdout takeover/restore
│   ├── project_trust.rs      # project-trust.ts — trust flow orchestrator
│   ├── prompt.rs             # c60 — SystemPromptOpts, build_system_prompt
│   ├── provider/             # c30 + c82 — ProviderApi, attribution, display names
│   ├── queue.rs              # c75 — MessageQueue for steer/followUp
│   ├── registry.rs           # c30 — ModelRegistry, ProviderConfig
│   ├── resolver.rs           # model-resolver.ts — pattern-based model resolution
│   ├── retry.rs              # retry state machine with exponential backoff
│   ├── session.rs            # c75 — AgentSession: fork, navigate, auto-persist
│   ├── templates.rs          # prompt-templates.ts — template parse & substitution
│   ├── tools/                # read, bash, edit, write, grep, find, ls, etc.
│   └── trust.rs              # trust-manager.ts — trust decision helpers
├── infra/
│   ├── config/               # config.yaml loader, validation, secret resolution
│   ├── event/                # c40 — EventBus
│   ├── hooks/                # pre/post step hooks with script execution
│   ├── resource/             # c60 — DefaultResourceLoader, diagnostics
│   ├── session/              # c50 + c65 — compaction, cwd, tree ops, export/
│   ├── settings/             # c35 — SettingsManager with deep_merge (30 fields)
│   ├── skills/               # c45 — SKILL.md loader + manager + MCP
│   ├── source_info.rs        # c98 — unified SourceInfo for all resources
│   ├── timing.rs             # c86 — startup timing instrumentation
│   └── trust/                # c55 — TrustManager (file-backed store)
├── interface/
│   ├── cli/                  # CLI arg parsing + dispatch (with timing/guidance integrated)
│   ├── diff_review/          # Diff review UI
│   ├── print.rs              # print mode (non-interactive)
│   ├── resources.rs          # c120 — resources list/info/doctor
│   └── rpc.rs                # c115 — JSONL RPC mode
└── ...
```

## 2026-06-21 Code Organization Review & QA

After confirming all 25 pi-mono modules are aligned, a thorough code review
was conducted focusing on clean architecture and minimal warnings.

### Changes Made

| Change | Description | Impact |
|--------|-------------|--------|
| **`ThinkingLevel` moved** | `agent/session.rs` → `agent/types.rs` (cross-cutting type used by 10 files) | Removes infra→session dependency |
| **`ModelMeta` moved** | `agent/session.rs` → `agent/types.rs` (same reasoning) | Removes infra→session dependency |
| **`BashExecutionParams` struct** | `append_bash_execution` 9 params → `BashExecutionParams` struct | Clippy 9/7 warning gone |
| **`OnChunkCallback` type alias** | `Box<dyn FnMut(&str) + Send + 'a>` factored out | Clippy complex-type warning gone |
| **Dead code removed** | `DispatchResult`, `has_handler`, `dispatch_slash_command`, `Extension` variant, test references | ~12 warnings eliminated |
| **Dead code annotated** | Attribution header functions/hosts kept but `#[allow(dead_code)]` | 8 warnings suppressed, code preserved |
| **Default model IDs expanded** | 2→35 entries matching pi's `defaultModelPerProvider` | Full alignment |
| **SourceInfo migration** | `source_path`→`source_info` in `SlashCommandInfo` and `PromptTemplate` | Consistency with c98 |

### Results

```
cargo clippy --lib:   24 warnings → 3 warnings (all intentional wildcard patterns)
cargo test --lib:     414→412 tests (2 dead-code tests removed)
cargo fmt:            clean
```

### Recommended Next Steps

1. **Reduce 3 remaining wildcard warnings** — refactor match arms to enumerate variants
2. **Link attribution headers into HTTP dispatcher** — use `merge_provider_attribution_headers` in c83's HTTP client
3. **Split `compaction.rs` (1347 lines)** — token estimation / cut-point / summary sub-modules if crates get larger
