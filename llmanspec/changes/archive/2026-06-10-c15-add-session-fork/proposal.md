---
change_id: c15-add-session-fork
title: "实现会话分叉与树形导航"
depends_on: [c08-add-llm-compaction]
status: active
priority: 15
---

# 变更提案：会话分叉

## Why

当前 SessionManager 的 `create()` 接受 `parent_session` 参数但未实现分叉逻辑。fork 是 LLM coding agent 的核心功能 — 用户回答不同分支的 "A 或 B" 问题时需要从分支点创建子会话。

pi 的 session fork 模式：
1. `SessionManager.fork(parentId, atEntryId)` — 复制父会话条目到分叉点 + 追加 branch_summary 条目 + 复制 settings
2. `branch_summary` 条目类型将父会话中被跳过的条目摘要化
3. Session tree: 父-子链接允许导航和上下文桥接

## What Changes

### SessionManager::fork()

```rust
pub async fn fork(
    &self,
    parent_id: &str,
    child_id: &str,
    at_entry_id: &str,
) -> Result<(), String>
```

1. 加载父会话的所有条目
2. 找到 `at_entry_id` 的索引
3. 复制条目 [0..index] 到新会话文件
4. 如果 index < entries.len()（剩余条目存在），生成 branch_summary 描述被跳过的条目
5. 创建子会话 header（带 `parent_session` 字段）
6. 用正确的 parent_id 写入所有条目

### Branch Summary

`generate_branch_summary(parent_entries: &[SessionEntry]) -> String` — 简单摘要：
- 条目数量和类型分布
- 最后一条用户消息
- 突出的操作（工具调用数、已编辑文件）
- 不调用 LLM；这由 c08 的 LLM compaction 覆盖

### AgentIntegration

`AgentSession::fork_session(at_entry_id) -> Result<String>` — 创建子会话 ID 并 fork。

## Capabilities

- **session-persistence**: 会话分叉与分支摘要

## Impact

- `src/infra/session/manager.rs`: 新增 `fork()` 方法
- `src/infra/session/types.rs`: 无变更（BranchSummaryEntry 已存在）
- `src/agent/session.rs`: 新增 `fork_session()` 方法
- `tests/features/session.feature`: 已有 fork 场景
