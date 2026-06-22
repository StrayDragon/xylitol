---
id: c195-wire-compaction-lifecycle
title: "Wire Compaction into Session Lifecycle — auto-compaction, branch summarization, file tracking"
depends_on: [c170-refactor-agent-message-types, c180-rebuild-agent-session]
---

## Why

当前压缩模块（`src/infra/session/compaction.rs`）有 1400+ 行代码实现了剪切点检测、摘要生成的骨架，但缺少以下关键功能：

1. **未接入 AgentSession 生命周期**：自动压缩/手动压缩从未实际触发
2. **无分支摘要**：当用户 fork 分支时，未 fork 的分支内容应被摘要保存
3. **无文件操作跟踪**：pi 的 `CompactionDetails` 记录了 readFiles/modifiedFiles，用于压缩时保留文件操作历史
4. **无扩展驱动压缩**：pi 支持 `session_before_compact` 扩展事件，让扩展参与压缩决策
5. **无溢出恢复**：当 context overflow 发生时，pi 会自动尝试压缩 + 重试

## What Changes

1. **AgentSession 接入**：在 `_checkCompaction()` 中调用 `prepareCompaction()` → `compact()`，处理结果
2. **手动压缩**：`compact(customInstructions?)` 方法 → 断开 agent → abort → 压缩 → 重连
3. **溢出恢复**：检测 `isContextOverflow()` → 强制压缩 → retry
4. **分支摘要**：`collectEntriesForBranchSummary()` + `generateBranchSummary()` → LLM 调用
5. **文件操作跟踪**：从工具调用中提取 readFiles/modifiedFiles，在压缩 entry 中存储
6. **扩展驱动压缩**：`session_before_compact` 事件 → 扩展可以提供自定义摘要
7. **Token 预估完善**：基于消息类型的分层预估（图片 4800 字符、文字 char/4、thinking/toolCall 逐项计算）

## Capabilities

- compaction

## Impact

- `src/infra/session/compaction.rs`：添加分支摘要、文件操作跟踪、溢出检测
- `src/agent/session.rs`：添加 `compact()` / `_checkCompaction()` 方法
- `src/infra/session/types.rs`：File tracking 类型

## Definition of Done

- [ ] `_checkCompaction()` 在 AgentSession.agent_end 后自动触发
- [ ] `compact()` 手动压缩可用（断开→abort→压缩→重连）
- [ ] 溢出恢复：context overflow → 强制压缩 → retry
- [ ] 分支摘要：未 fork 分支可生成摘要
- [ ] 文件操作跟踪：readFiles/modifiedFiles 在压缩 details 中持久化
- [ ] `cargo test` 通过
