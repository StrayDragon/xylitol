# Xylitol — Strategic Direction

> Last updated: 2026-06-13 · 5 critical fixes done · Dead code triage next

## Core Positioning

**Xylitol = Minimalist Agent Runtime**

Xylitol is a lean, single-shot agent execution engine — ReAct loop + tools + CLI.
It is NOT a platform, NOT an orchestration layer, NOT a multi-user service.

## Current Status (2026-06-13)

### ✅ Core Complete — Feature Development Frozen

All planned features are implemented. No new capabilities will be added in the near term.

| Layer | Status |
|-------|--------|
| AgentSession + unified ModelRegistry | ✅ Done (deduplicated 06-13) |
| 7 built-in tools + OutputAccumulator | ✅ Done |
| Session persistence (JSONL, tree, fork) | ✅ Done |
| Hooks (pre/post, block/modify/allow) | ✅ Done |
| Compaction (LLM summary + split detection) | ✅ Done |
| System prompt (dynamic build) | ✅ Done |
| Streaming cancel (CancellationToken wired) | ✅ Done (fixed 06-13) |
| Multi-provider (OpenAI + Anthropic API) | ✅ Done (OpenAI uses async-openai 06-13) |
| ReAct loop (multi-turn correct) | ✅ Done (fixed 06-13) |
| BDD framework (77 scenarios, 322 total tests) | ✅ Done |

### ❌ Explicitly NOT Implementing

| Category | Reason |
|----------|--------|
| PackageManager detection (npm/pnpm/yarn/bun) | User manages dependencies |
| OAuth / auth-storage / token flows | User configures API keys manually |
| Extensions SDK / plugin system | Out of scope |
| Additional providers (Gemini, Ollama, etc.) | OpenAI-like + Anthropic-like only |
| Multi-modal inputs (images, documents) | Not planned |

### 🤔 Interaction Mode: TBD

Current: **CLI single-shot (`print` mode)**.
Future: TUI / GUI / Web / MCP server — **decision pending**. Do not implement until finalized.

## Provider Policy

- **OpenAI-compatible API**: uses `async-openai` crate with user-provided `base_url` + `api_key`
- **Anthropic-compatible API**: hand-rolled HTTP+SSE (same pattern, different API format)
- No built-in model lists, no auth flows, no token management, no provider auto-discovery

## What Xylitol Should NOT Do

- Build workflow orchestration (DAG, checkpoint, branching) — that's zirvox
- Implement multi-user auth/authorization — single-user runtime
- Add HTTP/WebSocket gateway — called BY gateways, not a gateway itself
- Build a web dashboard — TUI or IDE integration is the interface
- Manage multi-channel ingress (Feishu, etc.) — zirvox handles that
- Auto-install packages or manage package.json — user responsibility
- Manage OAuth tokens or API key storage beyond config file — user responsibility

## Current Phase: Dead Code Triage & Architecture Polish

### Phase Goals

1. **Delete dead code** — remove 13 modules (~3,100L, 103 tests) that are unused or never wired
2. **Clean up `#![allow(dead_code)]`** — 5 modules that are actually used need annotation fix
3. **Tighten visibility** — review `pub` vs `pub(crate)` (currently 133:365)
4. **Write `docs/architecture.md`** — SSOT for the cleaned codebase

### Dead Code Inventory (3,056L + 96 tests to remove)

| Module | Lines | Tests | Why remove |
|--------|-------|-------|------------|
| `agent/trust.rs` | 528 | 11 | Never called by session/loop |
| `agent/project_trust.rs` | 393 | 11 | Completely isolated |
| `agent/resolver.rs` | 496 | 14 | CLI uses simple model match |
| `agent/commands.rs` | 172 | 9 | process_prompt() never called |
| `agent/diagnostics.rs` | 205 | 6 | Never wired |
| `agent/output_guard.rs` | 170 | 8 | print.rs doesn't use it |
| `agent/event.rs` | 110 | 3 | Loop emits events directly |
| `agent/queue.rs` | 77 | 0 | Session field never used |
| `agent/defaults.rs` | 62 | 5 | Never consumed |
| `infra/resource.rs` | 358 | 9 | Session never loads AGENTS.md |
| `infra/session/fine_tune.rs` | 244 | 7 | Never wired |
| `infra/session/storage.rs` | 174 | 4 | Not used by manager |
| `infra/session/gc.rs` | 13 | 0 | Empty shell |
| `infra/session/config.rs` | 67 | 0 | Not wired |

### Mis-marked Modules (should keep, fix annotations)

| Module | Lines | Issue |
|--------|-------|-------|
| `agent/retry.rs` | 84 | Used by loop.rs, remove `#![allow(dead_code)]` |
| `agent/templates.rs` | 318 | Used by session.rs for /template expansion |
| `infra/config/secret.rs` | 135 | Used by loader.rs |
| `infra/config/template.rs` | 143 | Used by loader.rs |
| `infra/config/validate.rs` | 139 | Used by loader.rs |
| `infra/config/loader.rs` | 405 | Internal uses of dead_code, fix |

### Guiding Principles

- **Less is more**: remove code that isn't pulling its weight
- **Explicit over implicit**: favor plain function calls over deep abstraction chains
- **Test-first refactoring**: tests must pass before and after every change
- **Small, auditable diffs**: each commit should be reviewable in isolation

## Success Metrics

- [x] BDD tests all green (77/77)
- [x] 7 built-in tools with BDD coverage
- [x] 2 LLM providers (OpenAI + Anthropic)
- [x] 322 tests pass (245 lib + 77 BDD)
- [x] Feature development frozen
- [x] 0 clippy warnings
- [x] Critical bugs fixed (ReAct loop, CancellationToken, dual ModelRegistry)
- [x] OpenAI provider uses async-openai (not hand-rolled HTTP)
- [x] lspz/dap removed
- [ ] Dead code eliminated — 3,100L target
- [ ] `#![allow(dead_code)]` cleaned — 21→5 files
- [ ] Visibility tightened — pub vs pub(crate)
- [ ] `docs/architecture.md` written
