# Design — 固定输入框角标与按钮配色

## 角标

```
╭────────╮
│ line   │
❯ line  ◨
╰─ [▤ 摘要]╯
```

- 末行右 padding 替换为 ` ◨`；下边框保留 `╯`。
- 窄档末行右端为 `│`。

## 配色

- 新增 `chrome_button_paint()`，固定 `ThinkingBorderLevel::Off` 色。
- `❯` / `[▤ 摘要]` / 未激活 `◨` 使用该 paint；边框与角继续用 `operation_zone_paint()`。
- 激活 `◨` 用 `paint_chip(active)`。
