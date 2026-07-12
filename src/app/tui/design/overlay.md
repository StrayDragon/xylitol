---
version: "alpha"
name: "overlay"
description: "Optional short confirms only — prefer editor-slot selectors."
tokens_from: "../DESIGN.md"
components:
  overlay-border:
    textColor: "{colors.muted}"
  overlay-body:
    textColor: "{colors.on-surface}"
---

# Overlay

> Token 根源：`{colors.*}` → [`../DESIGN.md`](../DESIGN.md)。

## 产品偏好（2026-07-12）

**默认不要**用 capturing overlay 做交互。下列场景走 **editor 槽替换**（与 demo / 已落地产品一致）：

| 场景 | 用 |
|---|---|
| 会话树 / travel | [`session-tree.md`](./session-tree.md)（`TreeSelector` 换槽） |
| Trust / Ask / 多选 | [`trust-prompt.md`](./trust-prompt.md) · ChoicePrompt 换槽 |
| 命令板 / 设置 | SelectList / SettingsList 换槽（见 [`editor.md`](./editor.md)） |

包侧仍保留 `show_overlay` + focus-restore（c575 / D08）供引擎与极少数短确认；**产品 UX 不以 overlay 为主控件**。

## MUST（若仍用 overlay）

1. 仅用于确认框等**极短交互**：居中短面板 + `OverlayHandle`。
2. 命令面板 / 设置 / Ask / 树 **MUST NOT** 做成大 overlay 仪表盘。
