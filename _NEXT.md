# Xylitol — Strategic Direction

> Last updated: 2026-06-10 · c25/c26 archived · all P0/P1 gaps closed · pi alignment ~91%

## Core Positioning

**Xylitol = General-Purpose Agent Runtime**

Xylitol focuses on being the **best agent execution engine** — capable of coding, research, analysis, and any task that benefits from a ReAct loop with tool calling. It does NOT aim to be an orchestration platform — that role belongs to zirvox.

## Current Status (2026-06-10)

### ✅ 与 pi 核心功能对齐度: ~91%

所有 P0 和 P1 差距已关闭。TrustManager + ProjectTrust (P2) 也已完成。剩余为 P2/P3 级别。

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
| OutputGuard (stdout takeover/restore) | ✅ 完整 |
| AgentSession lifecycle (event bus, auto-persist, start_new/resume) | ✅ 完整 |
| Streaming cancel (grep/find) | ✅ 代码完成 |
| Multi-provider (OpenAI + Anthropic) | ✅ 完整 |
| TrustManager (trust.json + lock + ancestor walk) | ✅ 完整 |
| ProjectTrust (resolve override → store → policy → prompt) | ✅ 完整 |
| BDD framework (77 scenarios, 356 total tests) | ✅ 完整 |
| **PackageManager 检测** | ⬜ P2 |
| **OAuth / auth-storage** | ⬜ P2 |
| **Extensions SDK** | ⬜ P3 |

### ✅ All P0/P1 Complete

- **OutputGuard**: `src/agent/output_guard.rs` (126L) — global atomic flag, RAII guard, write_raw_stdout, safe_println
- **AgentSession lifecycle**: event bus lazy init, begin_turn/end_turn, start_new_session/resume_session with CWD validation, enter_print_mode/leave_print_mode
- **ModelRegistry + ModelResolver**: ProviderConfig, exact/fuzzy/alias, thinking suffix, fallback, auth guidance
- **ResourceLoader**: AGENTS.md/CLAUDE.md walk-up, templates, skills
- **PromptTemplate**: $1..$N, $@, ${N:-default}
- **SlashCommands**: 7 builtin commands, dispatch interception
- **OutputAccumulator**: rolling buffer, temp file spill
- **Defaults + Diagnostics**: centralized values + startup checks
- **TrustManager**: `src/agent/trust.rs` (500L) — TrustStore (JSON persistence, file lock, ancestor-walk lookup, set/set_many atomic), trust options builder
- **ProjectTrust**: `src/agent/project_trust.rs` (380L) — resolution pipeline (override → no-inputs-auto-trust → store → default-policy → UI-prompt → fallback-deny), DefaultProjectTrust (Always/Never/Ask)

## What Xylitol Should NOT Do

- Build workflow orchestration (DAG, checkpoint, branching) — that's zirvox
- Implement multi-user auth/authorization — xylitol is a single-user runtime
- Add HTTP/WebSocket gateway — it gets called BY gateways
- Build a web dashboard — TUI + IDE integration is the interface
- Manage multi-channel ingress (Feishu, etc.) — zirvox handles that

## Growth Path

### Phase 1 — Core Harden ✅ 100% Done

All P0/P1 items complete. Core is solid.

### Phase 2 — Beyond Coding (medium-term)

- [ ] Expand tool categories (web, data, API via MCP skills)
- [ ] Add more LLM providers (Gemini, Ollama, custom endpoints)
- [ ] Multi-modal inputs (images, documents)
- [ ] PackageManager detection (npm/pnpm/yarn/bun)

### Phase 3 — Agent-as-a-Service (long-term)

- [ ] Expose agent capabilities via ACP-over-HTTP or MCP server mode
- [ ] zirvox can discover and invoke xylitol as a "super tool"
- [ ] Profile-based agent selection
- [ ] Agent composition (sub-agent pattern)

## Success Metrics

- [x] BDD 测试全绿 (77/77)
- [x] 7 built-in tools 全部 BDD 覆盖
- [x] At least 2 LLM providers (OpenAI + Anthropic)
- [x] 356 tests pass (279 lib + 77 BDD)
- [x] OutputGuard: stdout takeover for print mode
- [x] AgentSession full lifecycle with auto-persist
- [x] LLM-based compaction fully functional
- [x] Session persistence survives process restart
- [x] SecurityEngine policies cover built-in tools
- [x] ProjectTrust full resolution pipeline (override/store/policy/prompt)
- [ ] Can be invoked by zirvox workflow (via ACP/MCP)
- [ ] At least 3 LLM providers
