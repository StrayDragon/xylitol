---
depends_on: [c08-add-llm-compaction, c10-add-streaming-cancel, c15-add-session-fork]
---

# Proposal: Phase 2 Core Gaps — 补齐与 pi 的核心差距

## Why

与 pi-mono 对照分析后，xylitol 约完成 45-50% 的核心功能。排除延后的 TUI 和 Extensions SDK，仍有大量基础设施缺口：

- **会话树形结构 (id/parentId)**：当前条目扁平列表，无法在同一会话文件中导航，无法正确重建消息上下文
- **buildSessionContext**：无树形结构意味着无法从会话文件重建 AgentMessage[]
- **事件订阅模型**：当前 AgentEventStream 只能单次消费；pi 支持多监听器 + disconnect/reconnect
- **自动压缩集成**：压缩代码存在但未在 Agent 循环中触发（阈值检测 / overflow recovery）
- **自动重试**：网络 5xx/overloaded/rate-limit 等 transient errors 未处理
- **队列管理 (steer/followUp)**：流式响应中无法排队新消息
- **系统提示词构建**：tools 动态变化时提示词未更新，是静态 Option<String>
- **LLM 分支摘要**：fork/navigateTree 的 branch_summary 当前为纯文本统计，pi 用 LLM 生成结构化摘要
- **会话统计/元数据**：缺少 session stats、rich list()、model/thinking change 追踪

这些缺口**互相依赖**：树形结构是 buildSessionContext + fork/navigateTree 的基础；事件订阅模型是 auto-compaction + auto-retry 的基础；自动压缩+重试是稳定性的基础。

## What Changes

全面补齐以下 9 个领域的核心差距：

1. **会话树形结构** — entries 添加 id/parentId，支持 branch/leaf 导航，buildSessionContext
2. **事件订阅重构** — AgentEventStream → subscribe/unsubscribe 多监听器模式
3. **自动压缩集成** — Agent 循环中接入 compaction 检查，overflow recovery + threshold
4. **自动重试** — transient error 检测 + 指数退避 + 最大重试次数
5. **队列管理** — steer/followUp 消息排队 + delivery
6. **系统提示词动态构建** — 根据活跃 tools/skills/context_files 动态构建
7. **LLM 分支摘要升级** — branch_summary 从纯文本升级为 LLM 结构化摘要
8. **会话统计与元数据** — getSessionStats，rich list()，model/thinking change 持久化
9. **BDD 全覆盖** — 每个新功能有对应 BDD 场景

## Capabilities

| Capability | Action | Reason |
|-----------|--------|--------|
| session-persistence | modify + add | 树形结构 + buildSessionContext + 元数据 + change tracking |
| agent-session | modify + add | 系统提示词、队列管理、会话统计、事件订阅集成 |
| agent-runtime | modify + add | 事件订阅模型重构、自动压缩集成、自动重试 |
| compaction | modify + add | LLM branch_summary 升级 |

## Impact

- **不兼容变更**: SessionEntry 类型新增 `id`/`parent_id` 字段；AgentEventStream API 变更为订阅模式
- **新增文件**: `src/agent/event.rs`（事件总线）、`src/agent/retry.rs`（自动重试）、`src/agent/queue.rs`（队列管理）
- **修改文件**: `src/infra/session/types.rs`, `manager.rs`, `src/agent/session.rs`, `loop.rs`
- **session JSONL 格式**: 每个 entry 新增 `id`/`parent_id` 字段（向后兼容 v3→v4 迁移）
