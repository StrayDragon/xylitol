# Xylitol — Strategic Direction

> Last updated: 2026-06-13 · Config unified · B + C phases done · YAML wiring next

## Core Positioning

**Xylitol = Minimalist Agent Runtime**

Xylitol is a lean, single-shot agent execution engine — ReAct loop + tools + CLI.
It is NOT a platform, NOT an orchestration layer, NOT a multi-user service.

## Current Status (2026-06-13)

### ✅ Core Complete — Feature Development Frozen

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
| YAML config loader (5-layer merge, template, schema) | ✅ Code complete |
| ModelKind/ModelConfig unified (no ProviderKind) | ✅ Done (06-13) |

### ❌ Explicitly NOT Implementing

| Category | Reason |
|----------|--------|
| PackageManager detection | User manages dependencies |
| OAuth / auth-storage / token flows | User configures API keys manually |
| Extensions SDK / plugin system | Out of scope |
| Additional providers (Gemini, Ollama, etc.) | OpenAI-like + Anthropic-like only |
| Multi-modal inputs (images, documents) | Not planned |

### 🤔 Interaction Mode: TBD

Current: **CLI single-shot (`print` mode)**.
Future: TUI / GUI / Web / MCP server — **decision pending**.

## Provider Policy

- **OpenAI-compatible API**: uses `async-openai` crate with user-provided `base_url` + `api_key`
- **Anthropic-compatible API**: hand-rolled HTTP+SSE
- No built-in model lists, no auth flows, no token management, no provider auto-discovery

## Current Phase: Wire YAML Config → CLI

### What's Already Built But Not Connected

The YAML config pipeline is fully coded but `interface/cli/mod.rs` skips it entirely:

| Component | Code | Status |
|-----------|------|--------|
| `ConfigPaths::discover()` | `paths.rs` | ✅ Walks CWD for `.xylitol/config.yaml` |
| `load_app_config()` | `loader.rs` | ✅ 5-layer merge + MiniJinja templates |
| `secret.env` loader | `secret.rs` | ✅ Dotenv + permission check |
| JSON Schema validation | `validate.rs` | ✅ Runtime schema check |
| `AppConfig::resolve_model()` | `types.rs` | ✅ Alias → `agent::model::ModelConfig` |
| `AppConfig::resolve_profile()` | `types.rs` | ✅ Agent profile resolution |
| CLI integration | `cli/mod.rs` | ❌ **Hardcodes env vars, never calls config** |

### What Needs To Happen

```rust
// cli/mod.rs today:
let mut model_registry = ModelRegistry::new();
for (provider, env_var, kind) in [...env vars...] { ... }

// Should be:
let app_config = load_app_config(None)?;
// For each model alias in app_config.model.models:
//   resolve_model(alias) -> agent::model::ModelConfig
//   register in ModelRegistry
// Fallback: env vars as before
```

### Dead Code: Kept for TUI

Per decision, pi-parity modules are preserved (not deleted now):
- `trust.rs` / `project_trust.rs` — trust decisions
- `commands.rs` / `resolver.rs` / `templates.rs` — interactive commands
- `output_guard.rs` / `event.rs` — UI output control
- `resource.rs` — project context loading

### Completed Today

| Phase | Result |
|-------|--------|
| B: Fix `#![allow(dead_code)]` | ✅ retry.rs + templates.rs now use targeted `#[allow]` |
| C: Unify ModelKind/ModelConfig | ✅ ProviderKind removed; ModelKind gets serde/schemars |
| C: Rename YAML `ModelConfig` → `ModelsConfig` | ✅ No name conflict with agent-level ModelConfig |

## Success Metrics

- [x] BDD tests all green (77/77)
- [x] 7 built-in tools with BDD coverage
- [x] 2 LLM providers (OpenAI + Anthropic)
- [x] 322 tests pass (245 lib + 77 BDD)
- [x] Feature development frozen
- [x] 0 clippy warnings
- [x] Critical bugs fixed (ReAct loop, CancellationToken, dual ModelRegistry)
- [x] OpenAI provider uses async-openai
- [x] lspz/dap removed
- [x] `#![allow(dead_code)]` fixed (retry.rs, templates.rs)
- [x] ModelKind/ModelConfig unified — single canonical enum + struct
- [ ] CLI wired to YAML config loader
- [ ] `AppConfig::model` field renamed to `models`
- [ ] Visibility tightened — pub vs pub(crate) review
- [ ] `docs/architecture.md` written
