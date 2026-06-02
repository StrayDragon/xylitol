# Xylitol — Strategic Direction

> Last updated: 2026-06-02

## Core Positioning

**Xylitol = General-Purpose Agent Runtime**

Xylitol focuses on being the **best agent execution engine** — capable of coding, research, analysis, and any task that benefits from a ReAct loop with tool calling. It does NOT aim to be an orchestration platform — that role belongs to [zirvox](../zirvox).

## What Xylitol Does Best

- **ReAct Agent Loop**: Clean, event-driven, trait-decoupled execution engine
- **Tool System**: `XyTool` trait with 8 built-in dev tools + MCP extensibility
- **Security Engine**: Fine-grained policy enforcement (regex/glob path restrictions, command filtering, resource limits)
- **Multi-Provider LLM**: `XyModel` trait with OpenAI + Anthropic, easy to extend
- **Agent Profiles**: YAML-driven multi-personality configuration
- **Multi-Interface**: TUI (ratatui), CLI (print mode), ACP (IDE integration)

## What Xylitol Should NOT Do

- Build workflow orchestration (DAG, checkpoint, branching) — that's zirvox
- Implement multi-user auth/authorization — xylitol is a single-user runtime
- Add HTTP/WebSocket gateway — it gets called BY gateways, it doesn't BE one
- Build a web dashboard — TUI + IDE integration is the interface
- Manage multi-channel ingress (Feishu, etc.) — zirvox handles that

## Growth Path

### Phase 1 — Coding Agent (current focus)

- [ ] Harden the existing tool system (bash, read, write, edit, grep, find, ls, patch)
- [ ] Improve SecurityEngine with granular per-tool policies
- [ ] Complete ACP server mode for IDE integration (Zed, VS Code)
- [ ] Stabilize session persistence beyond InMemorySession
- [ ] Polish TUX experience: markdown rendering, diff review, history navigation

### Phase 2 — Beyond Coding (medium-term)

- [ ] Expand tool categories:
  - Web tools (fetch, search, scrape) — via MCP skills
  - Data tools (CSV/JSON processing, chart generation)
  - API tools (REST client, GraphQL)
- [ ] Add more LLM providers (Gemini, local models via Ollama, custom endpoints)
- [ ] Integrate Planner into main loop (task decomposition → step execution)
- [ ] Add conversation memory / context compaction for long sessions
- [ ] Support multi-modal inputs (images, documents)

### Phase 3 — Agent-as-a-Service (long-term)

- [ ] Expose agent capabilities via ACP-over-HTTP or MCP server mode
- [ ] zirvox can discover and invoke xylitol as a "super tool" in workflows
- [ ] Profile-based agent selection: different profiles for coding, research, review, etc.
- [ ] Streaming results back to caller (zirvox Gateway → WebSocket → user)
- [ ] Agent composition: xylitol can call other xylitol instances (sub-agent pattern)

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
│  │  │ (LLM)    │  │  ├─ Built-in (8)  │ │    │
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

- [ ] Can be invoked by zirvox workflow as a Step (via ACP/MCP)
- [ ] IDE integration works end-to-end (Zed / VS Code via ACP)
- [ ] At least 3 LLM providers supported (OpenAI, Anthropic, +1)
- [ ] Tool count ≥ 15 (built-in + MCP skills)
- [ ] Session persistence survives process restart
- [ ] SecurityEngine policies cover all built-in tools
