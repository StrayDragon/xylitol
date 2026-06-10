# Design: LLM 上下文压缩

## 数据流

```
AgentLoop::run() → turn end
  → get_context_usage(messages, context_window, threshold)
  → shouldCompact? yes
  → compact_current_session(model)
    → SessionManager::load(session_id) → Vec<SessionEntry>
    → find_last_compaction() → previousSummary?
    → estimate_context_tokens(entries) → before_count
    → find_cut_point(entries, boundary_start, entries.len(), keep_recent_tokens)
    → collect messages_to_summarize (entries [boundary_start..cut_index])
    → extract_file_ops(messages_to_summarize, previous_compaction)
    → generate_summary(messages, model, reserve_tokens, previous_summary)
      → serialize_conversation(messages) → text
      → wrap in <conversation> tags
      → call model.generate() with summarization prompt
      → extract text content
    → append file ops XML tags to summary
    → build CompactionEntry { summary, first_kept_entry_id, tokens_before, details }
    → SessionManager::append(entry)
    → yield CompactionEnd event
```

## 关键设计决策

### 1. 使用 `XyModel.generate()` 非流式调用

摘要生成不需要流式输出。调用 `model.generate(vec![system_msg, user_msg], &[], false)` 然后等待完整结果。这避免了处理 streaming state machine 的复杂性。

### 2. TOON-based 摘要格式对齐 pi

保留 pi 的 6 段结构化格式而不引入 pi 专有的自定义指令。系统提示保持不变：
- `## Goal`
- `## Constraints & Preferences`
- `## Progress` (### Done / In Progress / Blocked)
- `## Key Decisions`
- `## Next Steps`
- `## Critical Context`

### 3. 切点检测：Walk backwards

从最新 entry 向后遍历：
1. 对每条 message entry 估算 token 数（字符/4）
2. 累计已遍历 tokens ≥ `keep_recent_tokens` 时停止
3. 从当前位置向前找到最近的 valid cut point（user/assistant，非 tool result）
4. 如果切点是一个 assistant 消息（切半个 turn），标记 isSplitTurn（本变更仅标记，暂不处理 prefix summarization）

### 4. 文件追踪：附加在末尾

用简单的 XML tags 文件清单添加到摘要末尾：
```
<read-files>
path/to/file.txt
</read-files>

<modified-files>
path/to/changed.rs
</modified-files>
```

从 `XyPart::FunctionCall { name, args }` 块中提取，if `name in {"read","write","edit"}` → `args.path`。

### 5. CompactionSettings

```rust
struct CompactionSettings {
    enabled: bool,          // from CompactionConfig.enabled
    reserve_tokens: u64,    // default 16384 (align pi)
    keep_recent_tokens: u64,// default 20000 (align pi)
}
```

### 6. 不做 split-turn 前缀摘要

Split-turn detection 代码会写并标记 cut_point_result.is_split_turn，但 `compact_session()` 不会做 any the additional LLM call for the turn prefix。这跟 `c15-add-session-tree` 一起处理。

## 风险

| 风险 | 缓解 |
|------|------|
| LLM 摘要调用失败 | 失败时回退到简单文本拼接 stub，不阻断 loop |
| 摘要 token 消耗过大 | reserve_tokens 限制 LLM 调用的 max_tokens（0.8x reserve） |
| Summary 包含错误信息 | 不是问题——这是 LLM 摘要的固有特性。后续可加 validation hook |
