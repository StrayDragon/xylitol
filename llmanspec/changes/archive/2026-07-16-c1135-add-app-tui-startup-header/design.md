# Design — c1135-add-app-tui-startup-header

## 已拍板

| 决策 | 选择 |
|---|---|
| 位置 | scrollback **上方**独立槽（品牌 + Skills/MCP） |
| 内容 | Codex 简约卡片：>_ xylitol + model/directory/skills/mcp（无 ASCII logo、无「木糖醇」）；卡内换行全量 |
| Prompt | **不做** |
| 截断 | **禁止 `...`**；按宽度换行 |
| 空资源 | 仍显示品牌行 |
| 密钥 | 只显示 server id / tool_count / 短诊断 |
| Driver 缝 | `loaded_resources_snapshot` |
| DESIGN | 允许紧凑品牌 + 换行清单；禁止键墙 |

## 布局

```text
loaded_resources   ← Codex 简约卡片（无省略号）
scrollback
queue
status
editor
footer
```

## /reload

`handle_reload` 在 `set_dollar_skill_catalog` 之后调用 `session.refresh_loaded_resources(driver)`。

## ScriptedDriver

可注入 skills 名 + MCP snapshot 供 harness。
