# Design: c1280 quiet tool chrome

## Decision

安静化落在 **bridge 摘要与 Tool.output 填充**，不改 Expandable 组件契约。

| 面 | 行为 |
|---|---|
| `args_preview` | write/edit/read/… 永不 `compact_json_preview(args)`；缺字段 → 短占位 |
| write 行数 | 有 `content` 字符串时附加 `(N lines)`（`lines().count()`，空串不计） |
| edit path | 顶层 `path`/`file`，否则 `edits[0].path`/`file` |
| 成功 output | write/edit 成功且结果为机器 JSON → `Tool.output` 置空；edit 仍 push Diff |
| 错误 | `is_error` 时保留 `result` 原文（可截断由 expandable 负责） |

## Header 形态

scrollback 已是 `{name} [state] {args_preview}`。write 摘要含 `write ` 前缀会重复——本 change **保留** `write {path}` / `edit {path}` 前缀以兼容 att13 人类摘要习惯（与 c1260 一致）；不在本 change 改 header 拼装。

## Non-goals

- 不改默认 `tools_expanded=false`
- 不引入第二套折叠快捷键
- 不改 tool 实现或 provider 结果形状
