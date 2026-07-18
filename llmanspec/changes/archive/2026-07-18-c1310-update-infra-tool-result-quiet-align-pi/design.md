# Design: c1310 tool result quiet

## Channel split（对齐 pi content vs details）

| 通道 | 内容 | 消费者 |
|---|---|---|
| tool result **text**（history → project_for_llm） | 短成功/失败句 | 模型 |
| UI details | `display_diff` / patch | TUI bridge（c1300） |

## Preferred shape（最小侵入）

**方案 A（推荐）**：`XyTool::execute` / `execute_as_parts` 仍可产出 parts；新增或约定：

- 发往 history 的 text part = 短句
- `ToolExecutionEnd`（或并行 UI payload）仍携带 `display_diff` 供 bridge

若当前只有单一 `String` result：在 ReAct 收口处拆分——UI 事件用完整 JSON，push history 前 strip 为短句（过渡）；再收敛到正式 details 字段。

**方案 B**：工具返回短句；bridge 从 session/tool 旁路读 diff——侵入更大，不作首选。

## Modify t6

原：MUST 同时返回 unified + display diff（易被解读为进 LLM）。

新：MUST **产生** unified 与 display diff 供 UI；发往模型的 tool result 正文 MUST 为短成功句，MUST NOT 嵌入完整 diff。

## write

成功短句含 path + bytes（或 lines）；失败保留错误原文。

## Non-goals

- 不截断 tool **call** 参数（content/edits 仍在 assistant 消息）
- 不改 compaction 切点规则（仍不切 tool results）
