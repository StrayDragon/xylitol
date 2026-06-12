# Xylitol — Strategic Direction

> Last updated: 2026-06-12 · Code audit: agent/ ✅, infra/ ✅, interface/ ✅

## Core Positioning

**Xylitol = Minimalist Agent Runtime**

Xylitol is a lean, single-shot agent execution engine — ReAct loop + tools + CLI.
It is NOT a platform, NOT an orchestration layer, NOT a multi-user service.

## Current Status (2026-06-11)

### ✅ Core Complete — Feature Development Frozen

All planned features are implemented. No new capabilities will be added in the near term.

| Layer | Status |
|-------|--------|
| AgentSession + ModelRegistry + ModelResolver | ✅ Done |
| 7 built-in tools + OutputAccumulator | ✅ Done |
| Session persistence (JSONL, tree, fork) | ✅ Done |
| Hooks (pre/post, block/modify/allow) | ✅ Done |
| Compaction (LLM summary + split detection) | ✅ Done |
| ResourceLoader (AGENTS.md walk-up, templates, skills) | ✅ Done |
| PromptTemplate + SlashCommands | ✅ Done |
| System prompt (dynamic build) | ✅ Done |
| OutputGuard (stdout takeover/restore) | ✅ Done |
| AgentSession lifecycle (event bus, auto-persist) | ✅ Done |
| Streaming cancel (CancellationToken) | ✅ Done |
| Multi-provider (OpenAI-like + Anthropic-like API) | ✅ Done |
| TrustManager + ProjectTrust | ✅ Done |
| BDD framework (77 scenarios, 328 total tests) | ✅ Done |

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

- **OpenAI-compatible API**: any endpoint that speaks OpenAI chat completions (user sets `base_url` + `api_key`)
- **Anthropic-compatible API**: any endpoint that speaks Anthropic messages (user sets `base_url` + `api_key`)
- No built-in model lists, no auth flows, no token management, no provider auto-discovery

## What Xylitol Should NOT Do

- Build workflow orchestration (DAG, checkpoint, branching) — that's zirvox
- Implement multi-user auth/authorization — single-user runtime
- Add HTTP/WebSocket gateway — called BY gateways, not a gateway itself
- Build a web dashboard — TUI or IDE integration is the interface
- Manage multi-channel ingress (Feishu, etc.) — zirvox handles that
- Auto-install packages or manage package.json — user responsibility
- Manage OAuth tokens or API key storage beyond config file — user responsibility

## Current Phase: Code Audit & Architecture Optimization

### Phase Goals

1. **Audit existing code** — identify dead code, redundant abstractions, inconsistency
2. **Fix clippy warnings** — resolve 27 pre-existing warnings
3. **Optimize architecture** — reduce coupling, improve module boundaries, simplify where possible
4. **Strengthen tests** — ensure test coverage quality, not just quantity
5. **Remove dead/vestigial code** — anything related to out-of-scope features

### Guiding Principles

- **Less is more**: remove code that isn't pulling its weight
- **Explicit over implicit**: favor plain function calls over deep abstraction chains
- **Test-first refactoring**: tests must pass before and after every change
- **Small, auditable diffs**: each commit should be reviewable in isolation

### Next Steps After Audit Completes

| # | Task | Status |
|---|------|--------|
| 1 | Audit interface/ — remove acp.rs, drop `#![allow(dead_code)]`, eliminate hardcoded model IDs | ✅ Done |
| 2 | Review `pub` vs `pub(crate)` visibility (335 pub : 1 pub(crate)) | ⬜ |
| 3 | Review dependency tree — remove unused, dedup | ✅ Done (32→31, deprecated yaml swapped) |
| 4 | Review `unsafe` usage (13 instances, all in tests) — add comments | ✅ Done (SAFETY comments + module docs) |
| 5 | Write `docs/architecture.md` as SSOT | ⬜ |

## Success Metrics (Updated)

- [x] BDD tests all green (77/77)
- [x] 7 built-in tools with BDD coverage
- [x] 2 LLM providers (OpenAI-like + Anthropic-like)
- [x] 328 tests pass (251 lib + 77 BDD)
- [x] Feature development frozen — no new capabilities in progress
- [x] 0 clippy warnings
- [x] Dead code eliminated — 296L removed (faux_provider, sse_mock, harness, make_compaction_entry)
- [x] agent/ layer audited — .unwrap() → .expect()
- [x] infra/ layer audited — .unwrap() → .expect()
