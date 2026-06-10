# Handoff: Phase 1 Core Harden

> 最后更新：2026-06-10 · c06/c07 已归档 · c08/c10/c15 已提案
> 上游参考：`../pi-mono` (pi coding-agent 源码)

## 当前 BDD 状态

```bash
cargo test --test bdd -- --test-threads=1
# → 77 passed, 0 failed ✅
```

| 指标 | 数值 |
|------|------|
| tests/bdd.rs | ~1450 行 |
| 步骤定义 | ~175 个 |
| 场景绑定 | 77 个跨 12 个 feature 文件 |
| BDD 框架 | rstest-bdd, 100% 通过 |

## 变更历史

| # | 变更 | 状态 | 说明 |
|---|------|------|------|
| c05 | rebuild-core | ✅ 归档 | 7 tools, ReAct loop, session, hooks, CLI |
| c06 | update-bdd-framework | ✅ 归档 | cucumber-rs → rstest-bdd |
| c07 | fix-bdd-scenarios | ✅ 归档 | 36 个 BDD 失败修复, 77/77 全绿 |
| c08 | add-llm-compaction | 🆕 提案 | LLM 结构化摘要 + 切点检测 + 文件追踪 |
| c10 | add-streaming-cancel | 🆕 提案 | grep/find 进程取消 |
| c15 | add-session-fork | 🆕 提案 | fork + branch_summary |

## 代码分析更新 (2026-06-10)

### Config: 比描述更完整
`src/infra/config/loader.rs` 已实现 **5 层 deep merge** + **minijinja 模板渲染** + **secret.env 加载**。

### Session flock: 降级 P2
pi 自身也没有显式 flock，仅用原子追加。单用户运行时足够。

### 当前 P0 变更

| 变更 | 范围 | 依赖 |
|------|------|------|
| c08-add-llm-compaction | LLM 摘要 + 切点检测 + 文件追踪 | 无 |
| c10-add-streaming-cancel | grep/find 进程 kill | 无 |
| c15-add-session-fork | fork + branch_summary | c08 |

## 文件结构概览

```
src/
  agent/          — agent loop, tools, config, prompts, session
  infra/          — hooks, security, skills, session, config
  interface/      — CLI/RPC and user-facing output
tests/
  bdd.rs          — BDD 步骤定义 (~1450 行, 77 个场景)
  features/       — 12 个 .feature 文件
llmanspec/
  changes/        — c08, c10, c15 (活跃) + archive/
  specs/          — 23 个能力规范
docs/
  testing.md      — 测试层级指南
```
