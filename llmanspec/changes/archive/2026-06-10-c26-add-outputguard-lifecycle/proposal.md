---
depends_on: []
---

# Proposal: OutputGuard + AgentSession 生命周期集成

## Why

c25 完成后，xylitol 与 pi 核心功能对齐度约 **85%**。剩余 2 个 P0 缺口影响 print 模式的完整性和 session 运行时健壮性：

1. **OutputGuard**：pi 的 `output-guard.ts` (108L) 实现了 stdout/stderr 劫持与恢复 — print 模式中禁止 agent/tool 向 stdout 打印（redirect 到 stderr），这样最终结果才能干净地写入 stdout。
2. **AgentSession 生命周期**：当前 `AgentSession` 缺少一些 pi 的关键集成点 — 事件总线未接入 session（turn 事件未自动持久化）、session 切换到新会话的动态 lifecycle 管理不完整。

此外，c08/c10/c15 三个变更**代码已完全实现**但从未创建 llman 变更工件进行追踪。本次提案将同步声明这些为事实上的已完成状态。

## What Changes

### P0: OutputGuard

| # | 模块 | 说明 | 行数 |
|---|------|------|------|
| 1 | `src/agent/output_guard.rs` | stdout/stderr takeover/restore | ~100L |
| 2 | `src/agent/session.rs` | `enter_print_mode()` / `leave_print_mode()` | ~30L |

逻辑（与 pi 对齐）：
- `take_over_stdout()` → 保存原 `stdout.write`，替换为 redirect 到 stderr
- `restore_stdout()` → 恢复原 `stdout.write`
- `write_raw_stdout(text)` → 绕过 takeover 直接写原始 stdout（用于 print 模式的最终输出）
- `is_stdout_taken_over()` → 检查当前状态

### P0: AgentSession 生命周期增强

| # | 模块 | 说明 | 行数 |
|---|------|------|------|
| 3 | `src/agent/session.rs` | 集成 `AgentEventBus`：turn start/end 事件 + 自动持久化 | ~60L |
| 4 | `src/agent/session.rs` | `start_new_session()` / `resume_session()` 方法 | ~40L |

### 声明：c08/c10/c15 事实完成

| 变更 | 实现位置 | 行数 | 说明 |
|------|---------|------|------|
| c08 | `infra/session/compaction.rs` | 1217 | `compact_session`, `generate_summary`, `find_cut_point`, `generate_branch_summary_llm` |
| c10 | `tools/grep.rs:90-131` + `tools/find.rs:67-105` | ~70 | `CancellationToken` + `tokio::select!` 进程 kill |
| c15 | `infra/session/manager.rs::fork()` + `agent/session.rs::fork_session()` | ~150 | fork with LLM branch summary |

## Capabilities

| Capability | Action |
|-----------|--------|
| agent-runtime | add: OutputGuard stdout takeover/restore |
| agent-session | add: AgentSession lifecycle (event bus integration, session switch) |

## Impact

- **新增文件**: `src/agent/output_guard.rs`
- **修改文件**: `src/agent/session.rs` (lifecycle 增强 + OutputGuard 集成)
- **向后兼容**: 完全兼容 — 新增功能通过 AgentSession 方法暴露
