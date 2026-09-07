# Design: c2600 观测双会话 id

## Decision

两套 id 都是一等属性，不靠 Langfuse 单一 `session.id` 兼差：

| 键 | 含义 | langfuse.session.id |
|---|---|---|
| `xylitol.session.bookmark_id` | 书签（JSONL / 用户切换） | **同一值**（产品 Session = 一本） |
| `xylitol.session.llm_id` | LLM 投影 / 供应商会话身份 | 不占用 |

B 未落地时 `llm_id` MAY 暂等于 bookmark_id，但键 MUST 仍写出（分析侧能发现「尚未分叉策略」）。B 落地后投影 id 按命名策略填，本刀不改策略表。

树边只在子本（create 时带 parent）写出 parent_bookmark_id + fork_entry_id。

无旧键兼容窗。

## Non-goals

- 并发槽（A / c2590）
- OpenCode header / prompt_cache_key 算法（B）
- COW 盘格式
