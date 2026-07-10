# Design — c466 ExpandableOutput

## 形态（对齐 pi bash/tool）

| 态 | 行为 |
|---|---|
| collapsed | wrap-aware 保留末尾 `max_preview_lines`；上方 dim `... (N earlier lines, ctrl+o to expand)` |
| expanded | 全文（仍 wrap） |
| streaming | 两态均可；collapsed 贴尾，N 随新行涨 |

`TruncateFrom::Head` 保留首 N + `more lines` 提示（read/grep 风格）；bash/tool 用 Tail。

## 分层

- **包**：纯渲染 + Component；不绑 Ctrl+O。
- **Host**：全局 `tools_output_expanded`；树开时 Ctrl+O 不进视口（c456）。

## 与 Alt+E

- Alt+E：块有无详情（summary vs detail present）
- Ctrl+O：详情已在时的视口高度
