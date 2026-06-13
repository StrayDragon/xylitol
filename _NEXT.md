# Xylitol — Strategic Direction

> Last updated: 2026-06-13 · YAML config wired ✅ · All phases complete

## Core Positioning

**Xylitol = Minimalist Agent Runtime**

Xylitol is a lean, single-shot agent execution engine — ReAct loop + tools + CLI.
It is NOT a platform, NOT an orchestration layer, NOT a multi-user service.

## Current Status (2026-06-13)

### ✅ Core Complete — Feature Development Frozen

| Layer | Status |
|-------|--------|
| AgentSession + unified ModelRegistry | ✅ Done |
| 7 built-in tools + OutputAccumulator | ✅ Done |
| Session persistence (JSONL, tree, fork) | ✅ Done |
| Hooks (pre/post, block/modify/allow) | ✅ Done |
| Compaction (LLM summary + split detection) | ✅ Done |
| System prompt (dynamic build) | ✅ Done |
| Streaming cancel (CancellationToken wired) | ✅ Done |
| Multi-provider (OpenAI + Anthropic API) | ✅ Done (async-openai) |
| ReAct loop (multi-turn correct) | ✅ Done |
| BDD framework (77 scenarios, 322 tests) | ✅ Done |
| YAML config → CLI wiring | ✅ Done (5-layer merge + template + schema) |
| ModelKind/ModelConfig unified | ✅ Done (no ProviderKind) |

### ❌ Explicitly NOT Implementing

| Category | Reason |
|----------|--------|
| PackageManager detection | User manages deps |
| OAuth / auth-storage | User sets API keys |
| Extensions SDK | Out of scope |
| Extra providers (Gemini, Ollama) | OpenAI+Anthropic only |
| Multi-modal | Not planned |

### 🤔 Interaction Mode: TBD

Current: **CLI single-shot (`print`)**. Future: TUI/GUI/MCP — pending decision.

## Config: How It Works

```
config.yaml (global) → config.local.yaml → project .xylitol/config.yaml → .local → --config
                                    ↓
                          load_app_config()
                                    ↓
                    ModelsConfig → ModelEntry → ModelMeta → ModelRegistry
                                                          (fallback: env vars)
```

Minimal `~/.config/xylitol/config.yaml`:
```yaml
model:
  default_model: gpt-4o
  models:
    gpt-4o:
      provider: openai
      model: gpt-4o
    sonnet:
      provider: anthropic
      model: claude-sonnet-4-20250514
      context_window: 200000
```

## Remaining Polish Items

| # | Task | Priority |
|---|------|----------|
| 1 | `AppConfig::model` → rename field to `models` | 🟡 |
| 2 | `pub` → `pub(crate)` tighten | 🟡 |
| 3 | `docs/architecture.md` | 🟡 |

## Success Metrics

- [x] BDD tests 77/77
- [x] 7 tools with BDD coverage
- [x] 2 LLM providers
- [x] 322 tests (245 lib + 77 BDD)
- [x] 0 clippy warnings
- [x] ReAct loop multi-turn correct
- [x] OpenAI uses async-openai
- [x] lspz/dap removed
- [x] ModelKind unified
- [x] YAML config wired
- [ ] `AppConfig::model` → `models` rename
- [ ] visibility tightened
- [ ] `docs/architecture.md`
