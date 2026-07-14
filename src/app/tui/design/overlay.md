---
version: "alpha"
name: "overlay"
description: "Engine OverlayHandle remains; product UX prefers editor-slot selectors. Playground showcase removed."
tokens_from: "../DESIGN.md"
---

# Overlay

> Token 根源：`{colors.*}` → [`../DESIGN.md`](../DESIGN.md)。

## 产品偏好

**默认不要**用 capturing overlay 做主交互。会话树 / Ask / `/model` / 命令板 / 设置走 **editor 槽替换**。

DESIGN playground **不再**提供 Overlay 静图槽（已移除 HTML/JS/CSS 演示）；引擎仍保留 `show_overlay` + focus-restore（c575 / D08）供极少数短确认。

| 场景 | 用 |
|---|---|
| 会话树 / travel | [`session-tree.md`](./session-tree.md) |
| Trust / Ask / 多选 | [`trust-prompt.md`](./trust-prompt.md) |
| 命令板 / 设置 | SelectList / SettingsList（[`editor.md`](./editor.md)） |

## MUST（若仍用 overlay）

1. 仅确认框等**极短交互**：居中短面板 + `OverlayHandle`。
2. 命令面板 / 设置 / Ask / 树 **MUST NOT** 做成大 overlay 仪表盘。
