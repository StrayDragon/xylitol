# Handoff: All P0/P1 Gaps Closed

> 最后更新：2026-06-10 · c25/c26 已归档 · 与 pi 核心对齐度 **~91%**
> 上游参考：`../pi-mono` (pi coding-agent 源码)

## 当前测试状态

```bash
cargo test --test bdd -- --test-threads=1  # → 77 passed ✅
cargo test --lib                             # → 259 passed ✅
just qa                                      # → fmt + clippy + test + doc + prek ✅
```

| 指标 | 数值 |
|------|------|
| lib tests | 279 |
| BDD scenarios | 77 |
| total | 356 |
| src 源文件 | 50+ 个，~18,200 行 |

## 变更历史

| # | 变更 | 状态 | 说明 |
|---|------|------|------|
| c05 | rebuild-core | ✅ 归档 | 7 tools, ReAct loop, session, hooks, CLI |
| c06 | update-bdd-framework | ✅ 归档 | cucumber-rs → rstest-bdd |
| c07 | fix-bdd-scenarios | ✅ 归档 | 36 个 BDD 失败修复, 77/77 全绿 |
| c25 | phase3-infra-gaps | ✅ 归档 | ModelRegistry/Resolver, ResourceLoader, PromptTemplate, SlashCommands, OutputAccumulator, defaults, diagnostics, SessionCWD |
| — | llm-compaction | ✅ 代码完成 | compaction.rs (1217L) — cut-point + LLM summary + fork |
| — | streaming-cancel | ✅ 代码完成 | grep/find CancellationToken + tokio::select! |
| — | session-fork | ✅ 代码完成 | SessionManager::fork() + LLM branch summary |
| c26 | add-outputguard-lifecycle | ✅ 归档 | OutputGuard (takeover/restore/RAII guard), AgentSession lifecycle (event bus, begin_turn/end_turn, start_new/resume_session) |

## 活跃变更: 无

## 与 pi 核心功能对比

### ✅ 已完成 (功能对齐 ~89%)

| pi 模块 | xylitol 对应 | 状态 |
|---------|-------------|------|
| model-registry.ts | registry.rs (435L) | ✅ |
| model-resolver.ts | resolver.rs (495L) | ✅ |
| resource-loader.ts | infra/resource.rs (357L) | ✅ |
| prompt-templates.ts | templates.rs (315L) | ✅ |
| slash-commands.ts | commands.rs (170L) | ✅ |
| output-accumulator.ts | tools/accumulator.rs (312L) | ✅ |
| session-cwd.ts | manager.rs::assert_session_cwd_exists | ✅ |
| defaults.ts | defaults.rs (61L) | ✅ |
| diagnostics.ts | diagnostics.rs (203L) | ✅ |
| auth-guidance.ts | registry.rs::auth_guidance_message | ✅ |
| output-guard.ts | output_guard.rs (126L) | ✅ |
| agent-session.ts (event bus + lifecycle) | session.rs | ✅ |
| system-prompt.ts | prompt.rs (185L) | ✅ |
| session-manager.ts | infra/session/manager.rs | ✅ |
| compaction/* | infra/session/compaction.rs (1217L) | ✅ |
| event-bus.ts | event.rs | ✅ |
| hooks | infra/hooks/ | ✅ |
| skills | infra/skills/mod.rs | ✅ |
| trust-manager.ts | trust.rs (500L) | ✅ |
| project-trust.ts | project_trust.rs (380L) | ✅ |
| 7 built-in tools | tools/*.rs | ✅ |

### ⬜ 剩余 P2/P3 延后

| pi 模块 | 行数 | 原因 |
|---------|------|------|
| package-manager.ts | 2573 | 依赖检测/安装 — 独立大变更 |
| auth-storage.ts | 533 | OAuth token 持久化 |
| extensions/* / sdk.ts | ~1000 | 整体延后到 Extensions SDK 阶段 |
| keybindings.ts / footer-data-provider.ts | ~750 | TUI — 属于 zirvox |
| settings-manager.ts / resolve-config-value.ts | ~1450 | pi 配置层 — xylitol 配置系统不同 |

## 文件结构概览

```
src/
  agent/          — agent loop, model, registry, resolver, tools, templates, commands,
                    output_guard, defaults, diagnostics, session, prompt, event, queue,
                    trust, project_trust
  infra/          — hooks, skills, resource, session, config
  interface/      — CLI/RPC, diff_review, ACP
tests/
  bdd.rs          — BDD 步骤定义, 77 个场景绑定
  features/       — 12 个 .feature 文件
llmanspec/
  specs/          — 28 个 spec（含 model-registry, tool-system）
  changes/archive/— 20+ 个归档变更
```
