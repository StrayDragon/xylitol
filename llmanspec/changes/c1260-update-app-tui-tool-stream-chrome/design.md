# Design: c1260 app-tui tool stream chrome

## UI 结构（对齐 pi）

```text
chatContainer
  ├── AssistantMessageView   // text + thinking only
  └── ToolExecutionView×N    // keyed by tool_call_id
```

`message_update` / 等价事件：

- 未见过的 tool id → `insert ToolExecutionView(name, id, partial_args)`
- 已见过 → `update_args(partial_args)`
- `tool_execution_start` → `mark_running`
- `tool_execution_update` → append output
- `tool_execution_end` → final + status

## 摘要（M1）

| tool | collapsed 默认 |
|---|---|
| bash / shell | `$ {command}`（过长截断） |
| read | `read {path}` |
| ls | `ls {path}` |
| edit / write | `edit {path}` 或产品已有 diff 入口 |
| 未知 | `name` + 关键字段启发式；否则短 JSON |

展开（现有 Alt+E / Ctrl+O 语义可保留）：完整 args + 输出。

## flush

- 流式 assistant 文本与 thinking 仍按 MessageEnd / 回合边界提交；
- **禁止**在 ToolExecutionStart 时把未闭合的「假工具文本」策略——本波无 XML 抽取；原生路径下 text 与 tool 已分块。
