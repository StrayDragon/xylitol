# Handoff: P0/P1 Closed · TrustManager ✅ · pi alignment ~91%

> 最后更新：2026-06-10 · 最新 commit: `feat(trust)` · 与 pi 核心对齐度 **~91%**
> 上游参考：`../pi-mono` (pi coding-agent 源码)

## 当前测试状态

```bash
cargo test --test bdd -- --test-threads=1  # → 77 passed ✅
cargo test --lib                             # → 279 passed + 1 ignored ✅
cargo fmt -- --check                         # → clean ✅
cargo clippy --all-targets                   # → 0 errors, 27 warnings (all pre-existing) ✅
```

| 指标 | 数值 |
|------|------|
| lib tests | 279 |
| BDD scenarios | 77 |
| total | 356 |
| src 源文件 | ~70 个，~19,000 行 |

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
| — | feat(trust) | ✅ 提交 | trust.rs (529L) + project_trust.rs (394L), 20 tests |

## 活跃变更: 无

## 与 pi 核心功能逐模块对比

### ✅ 已完成对齐 (21 项)

| pi 模块 | pi 行数 | xylitol 对应 | xylitol 行数 | 状态 |
|---------|--------|-------------|-------------|------|
| agent-session.ts | 3135 | session.rs + infra/session/manager.rs | 730 + 811 = 1541 | ✅ |
| model-registry.ts | 1034 | registry.rs | 435 | ✅ |
| model-resolver.ts | 640 | resolver.rs | 495 | ✅ |
| resource-loader.ts | 1025 | infra/resource.rs | 357 | ✅ |
| session-manager.ts | 1567 | infra/session/manager.rs | 811 | ✅ |
| compaction/* (3 文件) | ~1000 | infra/session/compaction.rs | 1217 | ✅ |
| prompt-templates.ts | 284 | templates.rs | 315 | ✅ |
| trust-manager.ts | 229 | trust.rs | 529 | ✅ |
| project-trust.ts | 95 | project_trust.rs | 394 | ✅ |
| output-accumulator.ts | 222 | tools/accumulator.rs | 312 | ✅ |
| output-guard.ts | 108 | output_guard.rs | 126 | ✅ |
| slash-commands.ts | 41 | commands.rs | 170 | ✅ |
| defaults.ts | 3 | defaults.rs | 61 | ✅ |
| diagnostics.ts | 15 | diagnostics.rs | 203 | ✅ |
| system-prompt.ts | — | prompt.rs | 185 | ✅ |
| event-bus.ts | — | event.rs | — | ✅ |
| auth-guidance.ts | — | registry.rs::auth_guidance_message | — | ✅ |
| session-cwd.ts | — | manager.rs::assert_session_cwd_exists | — | ✅ |
| hooks | — | infra/hooks/ | — | ✅ |
| skills | — | infra/skills/ | — | ✅ |
| 7 built-in tools | ~1500 | tools/*.rs | ~2000 | ✅ |

### ⬜ 剩余 P2 待实现 (3 项)

| pi 模块 | pi 行数 | 原因 |
|---------|--------|------|
| package-manager.ts | 2573 | npm/pnpm/yarn/bun 依赖检测安装 — 独立大变更 |
| auth-storage.ts | 533 | OAuth token 持久化 — 需先实现 OAuth flow |
| extensions/* + sdk.ts | ~4500 | Extension SDK — 整体扩展体系 |

### ❌ 故意不在范围内 (设计取舍)

| pi 模块 / 层级 | 原因 |
|---|---|
| `modes/interactive/**` (大量 TUI 组件) | → zirvox |
| `packages/tui/**` (59 个 TS 文件) | → zirvox |
| `settings-manager.ts` + `resolve-config-value.ts` (~1450L) | xylitol 用 `infra/config/` 替代 |
| `keybindings.ts` + `footer-data-provider.ts` (~750L) | TUI 层 → zirvox |
| `packages/agent/**` | pi 的 agent 抽象层；xylitol 自实现 |
| `packages/ai/**` | pi 的 provider 层；xylitol 有 `agent/provider/` |
| `utils/**` (图片/clipboard/浏览器/HTML导出等) | 平台相关，精简 |
| `telemetry.ts` / `timings.ts` / `experimental.ts` | 不实现 |
| `migrations.ts` | xylitol 版本管理方式不同 |
| `cli/startup-ui.ts` / `cli/session-picker.ts` 等 | TUI 层 → zirvox |

## 架构对比

```
pi-mono (TypeScript)                 xylitol (Rust)
─────────────────────────            ─────────────────────────
packages/                            src/
├── coding-agent/                    ├── agent/          (核心运行时 ~10,000L)
│   ├── src/core/  (核心 ~25,800L)   │   ├── loop.rs        ReAct loop
│   │   ├── agent-session.ts         │   ├── session.rs     AgentSession + lifecycle
│   │   ├── session-manager.ts       │   ├── trust.rs       TrustStore ← NEW
│   │   ├── model-registry.ts        │   ├── project_trust  resolution ← NEW
│   │   ├── model-resolver.ts        │   ├── registry.rs    ModelRegistry
│   │   ├── compaction/              │   ├── resolver.rs    ModelResolver
│   │   ├── tools/                   │   ├── tools/         7 built-in tools
│   │   ├── extensions/              │   ├── provider/      OpenAI + Anthropic
│   │   ├── trust-manager.ts         │   ├── prompt.rs      system prompt builder
│   │   ├── project-trust.ts         │   ├── commands.rs    /slash commands
│   │   ├── package-manager.ts       │   ├── templates.rs   prompt templates
│   │   ├── auth-storage.ts          │   ├── output_guard   stdout takeover
│   │   ├── settings-manager.ts      │   ├── defaults.rs    centralized defaults
│   │   ├── sdk.ts                   │   ├── diagnostics    startup checks
│   │   └── ...                      │   ├── event.rs       event bus
│   ├── src/modes/ (TUI/print/rpc)   │   ├── queue.rs       message queue
│   ├── src/cli/                     │   └── ...
│   └── src/utils/                   ├── infra/           (基础设施 ~6,000L)
├── agent/        (agent 抽象层)     │   ├── hooks/         pre/post hooks
├── ai/           (provider 层)      │   ├── skills/       MCP skills
├── tui/          (TUI 组件库)       │   ├── config/       config system (取代 settings-manager)
                                     │   └── session/      compaction/storage/manager/gc
                                     └── interface/       (入口层 ~2,000L)
                                         ├── cli/           CLI args + dispatch
                                         ├── print.rs      print mode
                                         ├── acp.rs        ACP protocol
                                         └── diff_review/  diff review mode
```

## 文件结构概览

```
src/
  agent/          — loop, session, trust, project_trust, registry, resolver,
                    tools (7 built-in), templates, commands, output_guard,
                    defaults, diagnostics, system prompt, event, queue,
                    provider (OpenAI + Anthropic + mock + fake)
  infra/          — hooks (pre/post), skills (MCP), resource (AGENTS.md walk-up),
                    session (compaction, storage, manager, gc, fine_tune),
                    config (loader, paths, secret, template, validate)
  interface/      — cli, print, acp, diff_review
tests/
  bdd.rs          — BDD step definitions, 77 scenarios
  features/       — 12 .feature files
  support/        — test harness
llmanspec/
  specs/          — 28 specs
  changes/archive/— 28 archived changes
  changes/not-planning/ — 2 deferred changes
```

## xylitol 独有的优势模块

| 模块 | 说明 |
|---|---|
| `agent/provider/anthropic.rs` + `openai.rs` | 原生 Rust 多 provider |
| `agent/provider/mock.rs` + `fake.rs` | 测试专用 provider |
| `agent/profile.rs` | Agent 配置文件 |
| `agent/retry.rs` | 重试机制 |
| `infra/config/` (7 files) | 完整配置系统（取代 pi 的 settings-manager） |
| `infra/session/gc.rs` + `fine_tune.rs` | Session GC + 微调数据导出 |
| `interface/acp.rs` | ACP 协议（pi 无此协议） |
| `interface/diff_review/` (3 files, ~1800L) | 差异审查（pi 无独立模块） |
| **llmanspec/ SDD workflow** | 28 spec + 完整 SDD 流程 |
| **BDD framework** | 77 场景 rstest-bdd（pi 无 BDD） |
