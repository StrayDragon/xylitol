# Xylitol — Strategic Direction

> Last updated: 2026-06-15 · Code audit: **COMPLETE** ✅ · YAML config wired ✅

## Core Positioning

**Xylitol = Minimalist Agent Runtime**

Xylitol is a lean, single-shot agent execution engine — ReAct loop + tools + CLI.
It is NOT a platform, NOT an orchestration layer, NOT a multi-user service.

## Current Status (2026-06-15)

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
| Code audit | ✅ Done (all 3 layers + deps + visibility + unsafe) |
| Architecture doc | ✅ Done (docs/architecture.md, 17 chapters) |

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
models:
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

## Audit Phase: Complete ✅

| # | Task | Status |
|---|------|--------|
| 1 | Fix 27 clippy warnings | ✅ Done |
| 2 | Audit agent/ infra/ interface/ layers | ✅ Done |
| 3 | Remove dead code (296L + acp.rs) | ✅ Done |
| 4 | Review `pub` vs `pub(crate)` visibility | ✅ Done (556:365) |
| 5 | Review dependency tree | ✅ Done (32→31) |
| 6 | Review `unsafe` (13 instances, tests only) | ✅ Done (SAFETY comments) |
| 7 | Write `docs/architecture.md` | ✅ Done (330 lines, 17 chapters) |

## Remaining Polish Items

None — all items complete 🎉

## Success Metrics

- [x] BDD tests 77/77
- [x] 7 tools with BDD coverage
- [x] 2 LLM providers (async-openai + Anthropic)
- [x] 322 tests (245 lib + 77 BDD)
- [x] 0 clippy warnings
- [x] ReAct loop multi-turn correct
- [x] YAML config wired (5-layer merge)
- [x] lspz/dap removed
- [x] ModelKind unified
- [x] Dead code eliminated
- [x] Visibility audited (pub vs pub(crate))
- [x] unsafe reviewed + documented
- [x] docs/architecture.md written

---

## Next Phase: TBD
