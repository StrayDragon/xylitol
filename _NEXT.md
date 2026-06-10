# Xylitol — Strategic Direction

> Last updated: 2026-06-10 · c07-fix-bdd-scenarios: 77/77 BDD 全绿

## Core Positioning

**Xylitol = General-Purpose Agent Runtime**

Xylitol focuses on being the **best agent execution engine** — capable of coding, research, analysis, and any task that benefits from a ReAct loop with tool calling. It does NOT aim to be an orchestration platform — that role belongs to zirvox.

## Current Status (2026-06-10)

### ✅ Complete

- **ReAct Agent Loop**: clean, event-driven, trait-decoupled (`XyModel` + `XyTool`)
- **7 Built-in Tools**: read, write, edit, bash, grep, find, ls — 77 BDD scenarios passing
- **Session Persistence**: JSONL-based, create/append/load/list
- **Hooks System**: pre/post dispatch, block/modify/allow, timeout-kill
- **Compaction** (threshold detection): context usage + shouldCompact
- **Multi-Provider**: OpenAI + Anthropic via `XyModel` trait
- **CLI Interface**: clap-based, print mode
- **BDD Framework**: rstest-bdd, 77/77 scenarios, native `cargo test`

### 🔴 P0 Remaining

- **LLM-based Compaction**: summarization core (stub only)
- **Grep/Find/Bash streaming cancel**: currently tokio spawn + timeout

### 🟡 P1 Remaining

- **Config 3-tier merge + ENV interpolation**: currently global tier only
- **Session file locking (flock)**: not implemented
- **Session tree / fork**: stub only

### 🟢 P2/P3

- Parallel tool execution, system prompt building
- Interactive TUI (removed — belongs in zirvox)

## What Xylitol Should NOT Do

- Build workflow orchestration (DAG, checkpoint, branching) — that's zirvox
- Implement multi-user auth/authorization — xylitol is a single-user runtime
- Add HTTP/WebSocket gateway — it gets called BY gateways, it doesn't BE one
- Build a web dashboard — TUI + IDE integration is the interface
- Manage multi-channel ingress (Feishu, etc.) — zirvox handles that

## Growth Path

### Phase 1 — Core Harden (current focus)

- [ ] LLM-based Compaction (summarization via LLM call)
- [ ] Streaming cancel for bash/grep/find
- [ ] Config merge (global → project → user) with ENV interpolation
- [ ] Session flock + fork support

### Phase 2 — Beyond Coding (medium-term)

- [ ] Expand tool categories (web, data, API via MCP skills)
- [ ] Add more LLM providers (Gemini, Ollama, custom endpoints)
- [ ] Integrate Planner into main loop
- [ ] Conversation memory / context compaction for long sessions
- [ ] Multi-modal inputs (images, documents)

### Phase 3 — Agent-as-a-Service (long-term)

- [ ] Expose agent capabilities via ACP-over-HTTP or MCP server mode
- [ ] zirvox can discover and invoke xylitol as a "super tool" in workflows
- [ ] Profile-based agent selection
- [ ] Streaming results back to caller
- [ ] Agent composition (sub-agent pattern)

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
│  │         AgentLoop (ReAct)            │    │
│  │  ┌──────────┐  ┌───────────────────┐ │    │
│  │  │ XyModel  │  │  ToolRegistry     │ │    │
│  │  │ (LLM)    │  │  ├─ Built-in (7)  │ │    │
│  │  │ OpenAI   │  │  ├─ MCP Skills    │ │    │
│  │  │ Anthropic│  │  └─ Security Wrap │ │    │
│  │  │ Gemini?  │  │                   │ │    │
│  │  │ Ollama?  │  │                   │ │    │
│  │  └──────────┘  └───────────────────┘ │    │
│  │                                      │    │
│  │  ┌──────────┐  ┌───────────────────┐ │    │
│  │  │ Planner  │  │  SecurityEngine   │ │    │
│  │  │ (optional)│  │  (policy enforce)│ │    │
│  │  └──────────┘  └───────────────────┘ │    │
│  └──────────────────────────────────────┘    │
│                                              │
│  ┌──────────────────────────────────────┐    │
│  │  Profiles / Config / Sessions        │    │
│  └──────────────────────────────────────┘    │
└──────────────────────────────────────────────┘
```

## Shared Components to Extract

These should become shared crates usable by both xylitol and zirvox:

| Component | Crate | Rationale |
|-----------|-------|-----------|
| SecurityEngine | `xy-security` | zirvox needs automatic tool policy too |
| XyTool trait + registry | `xy-tool-core` | Reusable tool abstraction for any Rust agent project |
| Repeat detection | (part of `xy-security` or `xy-agent-core`) | General-purpose loop safety |

## Relationship with Zirvox

```
zirvox = "brain & central nervous system"
  - Schedules workflows, manages channels, handles auth
  - Does NOT execute agent tasks directly (long-term)

xylitol = "hands & safety"
  - Executes agent tasks with tool calling & security
  - Does NOT manage users, channels, or orchestration

Communication: ACP over HTTP / MCP protocol
  - zirvox discovers xylitol as an MCP server
  - zirvox workflows include "invoke xylitol agent" steps
  - xylitol streams results back to zirvox
```

## Success Metrics

- [x] BDD 测试全绿 (77/77)
- [x] 7 built-in tools 全部 BDD 覆盖
- [x] At least 2 LLM providers supported (OpenAI + Anthropic)
- [ ] Can be invoked by zirvox workflow as a Step (via ACP/MCP)
- [ ] IDE integration works end-to-end (Zed / VS Code via ACP)
- [ ] At least 3 LLM providers supported (OpenAI, Anthropic, +1)
- [ ] Tool count ≥ 15 (built-in + MCP skills)
- [ ] LLM-based compaction fully functional
- [ ] Session persistence survives process restart
- [ ] SecurityEngine policies cover all built-in tools
