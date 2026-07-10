---
version: "alpha"
name: "overlay"
description: "Short centered confirm overlays — not command-palette dashboards."
tokens_from: "../DESIGN.md"
components:
  overlay-border:
    textColor: "{colors.muted}"
  overlay-body:
    textColor: "{colors.on-surface}"
---

# Overlay

> Token 根源：`{colors.*}` → [`../DESIGN.md`](../DESIGN.md)。

## MUST

1. 仅用于确认框等**短交互**：居中短面板 + `OverlayHandle`。
2. 命令面板 / 设置 **MUST NOT** 做成大 overlay 仪表盘——走 editor 槽替换（见 [`editor.md`](./editor.md)）。
