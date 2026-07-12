# design — c476 live scrollback

## 原则

- **仍是引擎 scrollback**（att1 / att6）：不是独立 TranscriptView。
- **积木在包、状态在应用面**（att7）：Expandable 折叠态由产品持有。
- **照抄 demo 形态，不照抄 demo 脚本**（无 plate seed、无 `/theme`）。

## 渲染映射

| UiEntry | 包组件 / 行为 |
|---|---|
| User | glyph + Markdown；可选 `user_message_bg` |
| Assistant / streaming | Markdown |
| Thinking | Expandable；旁注 `(Ctrl+T)` |
| Tool | Expandable + `apply_background_to_line` 状态 tint；旁注 `(Alt+E)` |
| Diff | header tint + `Diff` 正文；旁注 `(Alt+E)` |
| System / Error | muted / error 一行 |

## 键位（本变更最小）

- `Ctrl+T` / `Alt+E`：仅切换对应块展开（与 demo 一致）
- slash / steer / abort → **c480**

## 布局

```
scrollback（多组件拼行）
status 0|1
────
editor | tree | (trust prompt = c490)
────
footer
```
