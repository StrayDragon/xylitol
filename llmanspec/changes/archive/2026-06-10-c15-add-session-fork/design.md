# Design: 会话分叉

## 数据流

```
AgentSession::fork_session(at_entry_id)
  → child_id = uuid::Uuid::new_v4()
  → SessionManager::fork(parent_id, &child_id, at_entry_id)
    → load parent entries
    → find index of at_entry_id
    → split entries into kept [0..index+1] and skipped [index+1..]
    → generate branch_summary for skipped entries (if any)
    → create child session header with parent_session link
    → write kept entries + branch_summary_entry (if any)
    → return child_id
```

## 分支摘要生成

当 fork 后存在被跳过的条目时生成简单分支摘要（纯文本，非 LLM）：

```
分支摘要:
- 跳过 N 条记录
- 包含: 2 条用户消息, 3 条助手消息, 5 次工具调用
- 最后一条用户消息: "帮我修复 bug"
- 涉及文件: src/main.rs, tests/test.rs
```

与 pi 不同，这**不调用 LLM**。LLM 分支摘要已由 c08 `compact_session` 覆盖（该函数产生 CompactionEntry）。`branch_summary` 角色是会话树导航的轻量级占位标识。

## 与 c08 的关系

c08-add-llm-compaction 实现 LLM 摘要管道。c15 在此基础上增加分叉基础设施。依赖方向：c15 → c08（c15 使用 c08 的 CompactionEntry 和 LLM 摘要管道进行可选的分支 LLM 摘要，但初始实现使用纯文本占位符）。

## 风险

| 风险 | 缓解 |
|------|------|
| 分叉并发写入 | 原子追加，每个会话独立文件 |
| 条目 ID 冲突 | 保留原始条目 ID；子会话引用父 UUID 空间 |
| 大型会话文件复制 | 通过 JSONL 行级追加复制；无内存问题 |
