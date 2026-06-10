# Xylitol — Strategic Direction

> Last updated: 2026-06-10 · c25 archived · c26 proposed · 与 pi 对齐度 ~88%

## Core Positioning

**Xylitol = General-Purpose Agent Runtime**

Xylitol focuses on being the **best agent execution engine** — capable of coding, research, analysis, and any task that benefits from a ReAct loop with tool calling. It does NOT aim to be an orchestration platform — that role belongs to zirvox.

## Current Status (2026-06-10)

### ✅ 与 pi 核心功能对齐度: ~88%

| 层次 | 状态 |
|------|------|
| AgentSession + ModelRegistry + ModelResolver | ✅ 完整 |
| 7 built-in tools + OutputAccumulator | ✅ 完整 |
| Session persistence (JSONL, tree, fork) | ✅ 完整 |
| Hooks (pre/post, block/modify/allow) | ✅ 完整 |
| Compaction (LLM summary + split detection) | ✅ 完整 |
| ResourceLoader (context, templates, skills) | ✅ 完整 |
| PromptTemplate + SlashCommands | ✅ 完整 |
| System prompt (dynamic build) | ✅ 完整 |
| Defaults + Diagnostics | ✅ 完整 |
| Multi-provider (OpenAI + Anthropic) | ✅ 完整 |
| BDD framework (77 scenarios) | ✅ 完整 |
| Streaming cancel (grep/find) | ✅ 代码完成 |
| **OutputGuard** (stdout takeover) | 🔴 提案中 (c26) |
| **AgentSession 生命周期** (event bus + auto-persist) | 🔴 提案中 (c26) |
| **PackageManager 检测** | ⬜ P2 |
| **TrustManager + project-trust** | ⬜ P2 |
| **OAuth / auth-storage** | ⬜ P2 |

### ✅ Complete

- **ReAct Agent Loop**: clean, event-driven, trait-decoupled (`XyModel` + `XyTool`)
- **7 Built-in Tools**: read, write, edit, bash, grep, find, ls — 77 BDD scenarios passing
- **Session Persistence**: JSONL-based, create/append/load/list/tree/fork
- **Hooks System**: pre/post dispatch, block/modify/allow, timeout-kill
- **Compaction**: threshold detection + LLM summary + split detection (c08)
- **Multi-Provider**: OpenAI + Anthropic via `XyModel` trait
- **CLI Interface**: clap-based, print mode
- **BDD Framework**: rstest-bdd, 77/77 scenarios
- **ModelRegistry**: ProviderConfig, auth check, diagnostics, default models
- **ModelResolver**: exact/fuzzy/alias matching, thinking suffix, fallback
- **ResourceLoader**: AGENTS.md/CLAUDE.md walk-up, templates, skills
- **PromptTemplate**: $1..$N, $@, ${N:-default}, /template:name
- **SlashCommands**: 7 builtin, dispatch interception
- **OutputAccumulator**: rolling buffer, temp file spill
- **Defaults + Diagnostics**: centralized values + startup checks

### 🔴 P0 Remaining (unblocks print mode)

1. **OutputGuard** (`src/agent/output_guard.rs`): stdout takeover/restore for print mode — **c26**
2. **AgentSession 生命周期**: event bus 集成 + auto-persist turn lifecycle — **c26**

### 🟡 P1 (complete, no llman artifacts)

- Streaming cancel for grep/find — 代码完成 (CancellationToken)
- LLM-based Compaction — 代码完成 (compaction.rs 1217L)
- Session fork — 代码完成 (SessionManager::fork)

### ⬜ P2/P3

- **PackageManager 检测** (2573L) — 独立大变更, 低耦合
- **TrustManager + project-trust** — 信任决策存储
- **OAuth / Login** — auth-storage (keytar/tar)
- **Scoped models** — Ctrl+P cycling with `--models` flag
- **Extensions SDK** — P3 整体延后
- **TUI / Keybindings** — 属于 zirvox
- **Parallel tool execution, system prompt 已完备**

## What Xylitol Should NOT Do

- Build workflow orchestration (DAG, checkpoint, branching) — that's zirvox
- Implement multi-user auth/authorization — xylitol is a single-user runtime
- Add HTTP/WebSocket gateway — it gets called BY gateways, it doesn't BE one
- Build a web dashboard — TUI + IDE integration is the interface
- Manage multi-channel ingress (Feishu, etc.) — zirvox handles that

## Growth Path

### Phase 1 — Core Harden ✅ 85% done

- [x] LLM-based Compaction (`infra/session/compaction.rs` 1217L, 代码完成无需 llman 工件)
- [x] Config merge (5-layer deep merge + template + secret.env)
- [x] Session tree / fork (SessionManager::fork, 代码完成无需 llman 工件)
- [x] Streaming cancel for grep/find (CancellationToken, 代码完成无需 llman 工件)
- [x] ModelRegistry + ModelResolver (c25)
- [x] ResourceLoader, PromptTemplate, SlashCommands (c25)
- [x] OutputAccumulator (c25)
- [x] Defaults, Diagnostics, SessionCWD (c25)
- [ ] **OutputGuard** (stdout takeover for print mode) ← **NEXT**
- [ ] **AgentSession lifecycle integration** (event bus + auto-persist)

### Phase 2 — Beyond Coding (medium-term)

- [ ] Expand tool categories (web, data, API via MCP skills)
- [ ] Add more LLM providers (Gemini, Ollama, custom endpoints)
- [ ] Multi-modal inputs (images, documents)
- [ ] PackageManager detection

### Phase 3 — Agent-as-a-Service (long-term)

- [ ] Expose agent capabilities via ACP-over-HTTP or MCP server mode
- [ ] zirvox can discover and invoke xylitol as a "super tool" in workflows
- [ ] Profile-based agent selection
- [ ] Streaming results back to caller
- [ ] Agent composition (sub-agent pattern)

## Immediate Next Steps

按优先级排序：

### 1. 实施 c26-add-outputguard-lifecycle
```bash
llman-sdd-apply c26-add-outputguard-lifecycle
```

### 2. 后续 P2 独立变更
- PackageManager 检测 (2573L pi) — 独立 `c<next>`
- TrustManager + project-trust — 独立 `c<next>`
- OAuth / Login — 独立 `c<next>`

## Architecture Target

```
┌─────────────────────────────────────────────┐
│             Calling Systems                  │
│  zirvox (workflow) / IDE (ACP) / TUI / CLI  │
└────────────────┬────────────────────────────┘
                 │  ACP / MCP / stdin
┌────────────────▼────────────────────────────┐
│            Xylitol Runtime                   │
│                                              │
│  ┌──────────────────────────────────────┐    │
│  │         AgentSession                  │    │
│  │  ┌──────────┐  ┌───────────────────┐ │    │
│  │  │ ModelReg │  │  ModelResolver    │ │    │
│  │  ├──────────┤  ├───────────────────┤ │    │
│  │  │ XyModel  │  │  ToolRegistry     │ │    │
│  │  │ (LLM)    │  │  ├─ Built-in (7)  │ │    │
│  │  │ OpenAI   │  │  ├─ MCP Skills    │ │    │
│  │  │ Anthropic│  │  └─ Security Wrap │ │    │
│  │  └──────────┘  └───────────────────┘ │    │
│  │                                      │    │
│  │  ┌──────────┐  ┌───────────────────┐ │    │
│  │  │ Prompts  │  │  OutputGuard      │ │    │
│  │  │ Templates│  │  (stdout takeover)│ │    │
│  │  │ Commands │  │                   │ │    │
│  │  └──────────┘  └───────────────────┘ │    │
│  │                                      │    │
│  │  ┌──────────┐  ┌───────────────────┐ │    │
│  │  │ Defaults │  │  Diagnostics      │ │    │
│  │  │ CWD val  │  │  ResourceLoader   │ │    │
│  │  └──────────┘  └───────────────────┘ │    │
│  └──────────────────────────────────────┘    │
│                                              │
│  ┌──────────────────────────────────────┐    │
│  │  Sessions / Compaction / Hooks       │    │
│  └──────────────────────────────────────┘    │
└──────────────────────────────────────────────┘
```

## Success Metrics

- [x] BDD 测试全绿 (77/77)
- [x] 7 built-in tools 全部 BDD 覆盖
- [x] At least 2 LLM providers supported (OpenAI + Anthropic)
- [x] 329+ tests pass (252 lib + 77 BDD)
- [ ] OutputGuard: stdout takeover for print mode
- [ ] AgentSession full lifecycle with auto-persist
- [ ] Can be invoked by zirvox workflow as a Step (via ACP/MCP)
- [ ] IDE integration works end-to-end (Zed / VS Code via ACP)
- [ ] At least 3 LLM providers supported (OpenAI, Anthropic, +1)
- [ ] LLM-based compaction fully functional (c08)
- [ ] Session persistence survives process restart
- [ ] SecurityEngine policies cover all built-in tools
