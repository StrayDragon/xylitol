# Design: c1790 rebuild Tool 单块

## 问题

```text
Live:  MessageUpdate/Start (upsert id=fc_…) → End (find_tool_mut + fill) → 1× Tool
Rebuild: toolCall part → Tool(id=fc_) ; toolResult → Tool(id=entry_uuid) → 2× Tool
```

JSONL/`toolCallId` 正确；缺口仅在 UI 投影。

## 方案（B）

在 `app/tui/bridge` 提供与 live End **同语义**的合并入口，例如：

- `apply_persisted_tool_result(entries, tool_call_id, name, raw_result, is_error, details…)`
  或 rebuild 专用 `project_path_entries_to_ui(entries) -> Vec<UiEntry>`，内部对 assistant parts 调现有 upsert 逻辑，对 toolResult 调合并逻辑。

`rebuild_scrollback_from_travel` **只**负责 clear + 调共享投影 + 不插导航 banner（banner 仍由 host 尾随）。

### 为何不 A-only / C / D

| | 否因 |
|---|---|
| A 仅 session_tree 临时 merge | 易与 End 的 humanize/MCP/diff 再分叉 |
| C 持久化 UI 快照 | 双真源 |
| D 改 wire 嵌套 result | 破 s19、动 LLM |

## 配对规则

1. 读 `toolCallId`（兼 `tool_call_id`）。
2. `find_tool_mut(entries, toolCallId)` 命中 → 填 output / is_error / done / display_diff（对齐 End）；保留已有 args_preview / write_content / path。
3. 未命中 → orphan：push 一条 done Tool，**id 仍用 toolCallId**（若有）否则 entry id。
4. 成功 write/edit 静默 JSON / display_diff 规则与 live 共用 helper。

## 非目标

- 改 `project_for_llm`、session store、provider adapter。
- activity-fold（c1760）。
