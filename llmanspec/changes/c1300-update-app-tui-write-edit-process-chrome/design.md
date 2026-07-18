# Design: c1300 write/edit process chrome

## Decisions

### Write body source

Streaming = **tool-call argument streaming**（与 pi 同），不是 `execute` 写盘流。

- `sync_tool_intent_from_message` / upsert：除 `args_preview` 外，write 把 `content` 写入 `Tool.output`（或专用 `body` 字段；优先复用 `output` 以免扩枚举，若语义冲突再拆字段）。
- 成功 End：保留 body（来自 args），quiet 掉机器 JSON（c1280 已有）；失败则末行错误。

### Viewport

| | write | bash（既有） | edit |
|---|---|---|---|
| 默认行数 | **10** Head | 5 Tail | **全量**（就绪即显） |
| Ctrl+O | 全量 content | 全量 output | n/a（或保留无操作） |
| Alt+E | 不挡 write body | 可挡其它 tool detail | **不挡** edit diff |
| 洗底 | **header+正文同一** pending/success/error 块 | 整块 | header tint；diff 正文不加 tool-*-bg（att4） |

### Edit merge

- `ToolExecutionEnd` edit 成功：把 `display_diff` 写入**同一** Tool 行（新字段或约定 output 形态），**不再** `push UiEntry::Diff`。
- 历史重建：session_tree 同规则。
- 修改 atb11：取消「必须另推 Diff」；改为「同块展示」。

### Status chrome

- Header：`{marker} {glyph} {name} {path-or-preview} (Alt+E?)` — **无** `[ok]`/`[err]`/`[…]`。
- Tint：pending / success / error（既有 tool-*-bg）。
- 失败：块末可见错误行。

### Path

- 相对 cwd 用 `~/` 或相对路径缩短（对齐 pi `shortenPath`）；绝对路径仅当在 cwd 外。

## Non-goals

- 不改 Expandable 包 API（除非 Head-10 hint 文案需对齐 pi `N more lines, TOTAL total`）
- 不在本 change 改 LLM tool_result 形状（c1310）
