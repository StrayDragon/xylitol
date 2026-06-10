---
change_id: c08-add-llm-compaction
title: "实现 LLM-based 上下文压缩（结构化摘要 + 切点检测）"
depends_on: []
status: active
priority: 8
---

# 变更提案：LLM 上下文压缩

## Why

当前 `compact_conversation()` 是纯文本拼接 stub — 仅将 old_turns 的 user messages 用 `\n- ` 连接，无 LLM 调用、无结构化摘要、无迭代更新、无切点检测。

pi 的 compaction 是完整 LLM 管道：
1. **shouldCompact** → 阈值检测 ✅ (已就绪)
2. **findCutPoint** → 在 session entries 中按 token 预算找到切点
3. **prepareCompaction** → 收集要压缩的 messages + 提取 file operations + 检测 split-turn
4. **generateSummary** → 调用 LLM 生成结构化摘要（Goal/Progress/Next Steps）
5. **compact** → 产出 CompactionEntry（summary + firstKeptEntryId + tokensBefore + details）

## What Changes

### 新能力：LLM summarization

| 组件 | 当前 | 目标 |
|------|------|------|
| 摘要生成 | `format!("[Compacted: {n} turns]")` | LLM 调用 `XyModel.generate()` 生成结构化摘要 |
| 摘要格式 | 自由文本 | 结构化：## Goal / ## Progress / ## Next Steps / ## Critical Context |
| 切点检测 | `turns.drain(..compact_boundary)` | 按 token 预算 + valid cut points 找到切点 |
| 迭代更新 | 无 | 带 `previousSummary` 作增量更新 |
| 文件追踪 | 无 | 跟踪 read/modified files 附在摘要末尾 |
| Split-turn | 无 | 切割到 assistant/tool 中间时保留 turn prefix |

### pi 对齐度

| pi 函数 | xylitol 目标 | 对齐 |
|---------|-------------|------|
| `findCutPoint(entries, start, end, keepRecent)` | `find_cut_point()` on `Vec<SessionEntry>` | 全文对齐 |
| `generateSummary(messages, model, reserveTokens, ...)` | `generate_summary()` via `XyModel.generate()` | 对齐 — 用 XyContent 替代 AgentMessage |
| `compact(preparation, model, ...)` | `compact_session()` 产出 `CompactionEntry` | SessionManager 已支持 |
| `prepareCompaction(pathEntries, settings)` | 集成在 compact_session 内 | 简化 — 单次调用 |
| Structured prompt | Goal/Progress/Next Steps/Critical Context | 相同格式 |
| File tracking | `<read-files>` / `<modified-files>` XML tags | 对齐 |
| Iterative summary update | 带 previous_summary 的 UPDATE_SUMMARIZATION_PROMPT | 对齐 |

### 不实现的部分（本变更范围外）

- Branch summarization — c15 session tree/fork 时实现
- Turn prefix summarization（split-turn 的次要部分）— defer
- File operation extraction from tool arguments — 为 split-turn 预留接口

## Capabilities

- **compaction**: LLM-based structured summarization + cut-point logic

## Impact

- `src/infra/session/compaction.rs`: 完全重写（~200 行 → ~400 行）
- `src/infra/session/manager.rs`: 新增 `write_entry()` 辅助方法
- `src/agent/session.rs`: 新增 `compact_current_session()` 方法集成 loop
- `tests/features/compaction.feature`: 已有 BDD 场景，需要实现步骤定义
- `tests/bdd.rs`: 新增 compaction 步骤定义
