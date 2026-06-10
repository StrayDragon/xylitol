# Xylitol — Strategic Direction

> Last updated: 2026-06-10 · c25 fully implemented · entering Phase 4: OutputGuard + AgentSession Integration

## Core Positioning

**Xylitol = General-Purpose Agent Runtime**

Xylitol focuses on being the **best agent execution engine** — capable of coding, research, analysis, and any task that benefits from a ReAct loop with tool calling. It does NOT aim to be an orchestration platform — that role belongs to zirvox.

## Current Status (2026-06-10)

### ✅ 与 pi 核心功能对齐度: ~85%

| 层次 | 状态 |
|------|------|
| AgentSession + ModelRegistry + ModelResolver | ✅ 完整 |
| 7 built-in tools + OutputAccumulator | ✅ 完整 |
| Session persistence (JSONL, tree, fork) | ✅ 完整 |
| Hooks (pre/post, block/modify/allow) | ✅ 完整 |
| Compaction (LLM summary + split detection) | ✅ 完整 (c08 待合并) |
| ResourceLoader (context, templates, skills) | ✅ 完整 |
| PromptTemplate + SlashCommands | ✅ 完整 |
| System prompt (dynamic build) | ✅ 完整 |
| Defaults + Diagnostics | ✅ 完整 |
| Multi-provider (OpenAI + Anthropic) | ✅ 完整 |
| BDD framework (77 scenarios) | ✅ 完整 |
| **OutputGuard** (stdout takeover) | 🔴 缺失 |
| **AgentSession 集成** (事件总线, 生命周期) | 🟡 部分 |
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

1. **OutputGuard** (108L in pi): stdout/stderr 劫持 — 进入 print 模式时暂停 TUI writer，恢复时接管。`src/agent/output_guard.rs`
2. **AgentSession 集成增强**: 事件总线自动持久化 turn 生命周期 (pi 的 agent-session.ts 中 auto-save、turn-reset 逻辑)

### 🟡 P1 (active proposals)

- **Streaming cancel for grep/find** — c10
- **LLM-based Compaction** — c08 (已实现, 待合并/归档)

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

- [x] LLM-based Compaction (c08 — implemented, not yet archived)
- [x] Config merge (5-layer deep merge + template + secret.env)
- [x] Session tree / fork support (c15 — proposed)
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

### 1. c25 归档
```bash
llman sdd archive c25-phase3-infra-gaps
```

### 2. 提案: OutputGuard + AgentSession 集成
新建变更 `c26-add-outputguard-session-lifecycle`:
- `src/agent/output_guard.rs`: stdout/stderr takeover (pi: output-guard.ts 108L)
- AgentSession 生命周期: event bus 持久化, turn-reset, auto-save
- 单元测试 + BDD

### 3. 继续推进 c08/c10/c15
这些提案需要实施和归档：
- c08-add-llm-compaction (LLM compaction 已实现)
- c10-add-streaming-cancel (grep/find 进程取消)
- c15-add-session-fork (fork + branch_summary)

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
