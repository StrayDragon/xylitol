# Handoff: Phase 3 Complete → Phase 4 P0 Gap

> 最后更新：2026-06-10 · c25 已归档 · c26 已提案
> 上游参考：`../pi-mono` (pi coding-agent 源码)

## 当前测试状态

```bash
cargo test --test bdd -- --test-threads=1  # → 77 passed, 0 failed ✅
cargo test --lib                             # → 252 passed, 0 failed ✅
just qa                                      # → fmt + clippy + test + doc + prek ✅
```

| 指标 | 数值 |
|------|------|
| lib tests | 252 (was 232 before c25) |
| BDD scenarios | 77 |
| src 源文件 | 50+ 个 |
| 总代码行数 | ~17,834 行 Rust |

## 变更历史

| # | 变更 | 状态 | 说明 |
|---|------|------|------|
| c05 | rebuild-core | ✅ 归档 | 7 tools, ReAct loop, session, hooks, CLI |
| c06 | update-bdd-framework | ✅ 归档 | cucumber-rs → rstest-bdd |
| c07 | fix-bdd-scenarios | ✅ 归档 | 36 个 BDD 失败修复, 77/77 全绿 |
| c25 | phase3-infra-gaps | ✅ 归档 | ModelRegistry/Resolver, ResourceLoader, PromptTemplate, SlashCommands, OutputAccumulator, defaults, diagnostics, SessionCWD |
| — | llm-compaction | ✅ 代码完成 | `infra/session/compaction.rs` (1217L) — LLM 摘要, cut-point, fork summary. 无 llman 工件。 |
| — | streaming-cancel | ✅ 代码完成 | grep/find CancellationToken + tokio::select! 进程 kill. 无 llman 工件。 |
| — | session-fork | ✅ 代码完成 | SessionManager::fork() + AgentSession::fork_session() + LLM branch summary. 无 llman 工件。 |
| c26 | add-outputguard-lifecycle | 🆕 提案 | OutputGuard + AgentSession 事件总线 + session lifecycle (P0) |

## c25 新增模块

| 模块 | 文件 | 行数 | pi 参照 |
|------|------|------|---------|
| ModelRegistry 升级 | `src/agent/registry.rs` | 435 | model-registry.ts (400L) |
| ModelResolver | `src/agent/resolver.rs` | 495 | model-resolver.ts (640L) |
| ResourceLoader | `src/infra/resource.rs` | 357 | resource-loader.ts (1025L) |
| PromptTemplate | `src/agent/templates.rs` | 315 | prompt-templates.ts (284L) |
| SlashCommands | `src/agent/commands.rs` | 170 | slash-commands.ts (41L) |
| OutputAccumulator | `src/agent/tools/accumulator.rs` | 312 | output-accumulator.ts (222L) |
| Defaults | `src/agent/defaults.rs` | 61 | defaults.ts (30L) |
| Diagnostics | `src/agent/diagnostics.rs` | 203 | diagnostics.ts (37L) |
| SessionCWD | `src/infra/session/manager.rs` (+40) | — | session-cwd.ts (59L) |
| AgentSession.prompt() | `src/agent/session.rs` (+60) | — | agent-session.ts (commands dispatch) |

## 与 pi 核心功能对比

### ✅ 已完成 (功能对齐)

| pi 模块 | xylitol 对应 | 状态 |
|---------|-------------|------|
| model-registry.ts | `registry.rs` | ✅ ProviderConfig, auth 检查, get_available, default models, diagnostics 收集 |
| model-resolver.ts | `resolver.rs` | ✅ 精确/模糊匹配, alias 优先, thinking-level 后缀, fallback |
| resource-loader.ts | `infra/resource.rs` | ✅ AGENTS.md/CLAUDE.md 向上查找, prompt templates 加载, SkillManager |
| prompt-templates.ts | `templates.rs` | ✅ $1/$N/$@/${N:-default}, /template:name 解析 |
| slash-commands.ts | `commands.rs` | ✅ BUILTIN_COMMANDS (7个), 命令检测, 参数提取, extension 注册 |
| output-accumulator.ts | `tools/accumulator.rs` | ✅ rolling buffer, temp file spill, snapshot |
| session-cwd.ts | `manager.rs::assert_session_cwd_exists()` | ✅ 目录存在验证 + fallback |
| defaults.ts | `defaults.rs` | ✅ DEFAULT_THINKING_LEVEL, DEFAULT_MAX_ITERATIONS, DEFAULT_COMPACTION_THRESHOLD |
| diagnostics.ts | `diagnostics.rs` | ✅ info/warning/error + collection |
| auth-guidance.ts | `registry.rs::auth_guidance_message()` | ✅ 缺失密钥引导信息 |
| tools/bash.ts | `tools/bash.rs` + accumulator | ✅ 流式输出缓冲, timeout, abort, truncate |
| tools/{read,write,edit,grep,find,ls}.ts | 对应 tools/*.rs | ✅ 全部 7 个内置工具 |
| system-prompt.ts | `agent/prompt.rs` (185L) | ✅ 动态构建, context files, skills |
| session-manager.ts | `infra/session/manager.rs` | ✅ JSONL CRUD, 树导航, fork, compaction |
| compaction/* | `infra/session/compaction.rs` | ✅ LLM 摘要 + 切点检测 (c08) |
| event-bus.ts | `agent/event.rs` | ✅ EventEmitter 等价 |
| messages.ts | `agent/types.rs` | ✅ XyContent, XyPart, XyRole |
| hooks | `infra/hooks/` | ✅ pre/post dispatch, block/modify/allow |
| skills | `infra/skills/mod.rs` | ✅ load/activate/deactivate/allowed_tools |

### 🔴 剩余 P0 差距 (影响功能完整性)

| pi 模块 | 行数 | 说明 | 建议变更 |
|---------|------|------|---------|
| **output-guard.ts** | 108 | stdout/stderr 劫持与恢复 (print 模式) | `c<next>` |
| **exec.ts** | 107 | 执行生命周期包装 (bash + diff review loop) | `c<next>` |
| **retry/backoff** | — | `agent/retry.rs` 已存在但未接入 session | 接入 |

### 🟡 剩余 P1 差距 (中优先级)

| pi 模块 | 行数 | 说明 | 建议变更 |
|---------|------|------|---------|
| **package-manager.ts** | 2573 | 依赖检测/安装/更新 (npm/pnpm/yarn/bun) | 独立变更 |
| **trust-manager.ts** + **project-trust.ts** | 229+95 | 项目信任决策存储 (lockfile) | 独立变更 |
| **auth-storage.ts** | 533 | OAuth token 持久化 (keytar/tar) | 独立变更 |
| **event-bus → AgentSession 集成** | — | turn 生命周期, 事件自动持久化 | session 增强 |
| **scoped-models** | — | Ctrl+P model cycling with `--models` flag | session 增强 |
| **messages.ts transformers** | 195 | compaction/branch summary 前缀/后缀 | session 增强 |

### ⬜ P2/P3 (低优先级或延后)

| pi 模块 | 原因 |
|---------|------|
| extensions/*, agent-session-runtime.ts, agent-session-services.ts, sdk.ts | 依赖 Extensions SDK (P3 整体延后) |
| keybindings.ts, footer-data-provider.ts | TUI 模式 — 属于 zirvox |
| http-dispatcher.ts | pi 的 RPC dispatch — xylitol 走 ACP |
| provider-attribution.ts, telemetry.ts, timings.ts, experimental.ts, source-info.ts | P3 次要功能 |
| settings-manager.ts (1165L), resolve-config-value.ts (286L) | pi 内部配置层 — xylitol 配置系统不同 |

## 当前 P0 变更 (按优先级)

| 优先级 | 变更 | 范围 | 依赖 |
|--------|------|------|------|
| 1 | c08-add-llm-compaction | LLM 摘要 + 切点检测 + 文件追踪 | 无 |
| 2 | c10-add-streaming-cancel | grep/find 进程 kill | 无 |
| 3 | c15-add-session-fork | fork + branch_summary | c08 |
| — | c25-phase3-infra-gaps | 此次变更, 待归档 | 无 |

## 文件结构概览

```
src/
  agent/          — agent loop, model, registry, resolver, tools, templates, commands, defaults, diagnostics, session, prompt
  infra/          — hooks, skills, resource, session, config
  interface/      — CLI/RPC, diff_review, ACP
tests/
  bdd.rs          — BDD 步骤定义, 77 个场景绑定
  features/       — 12 个 .feature 文件
llmanspec/
  changes/        — c08, c10, c15, c25 (活跃) + archive/
  specs/          — 23 个能力规范
```

## 配置系统

`src/infra/config/loader.rs` 已实现 **5 层 deep merge** + **minijinja 模板渲染** + **secret.env 加载**。
`SessionManager` 用原子追加写入, 无显式 flock (pi 同理)。
