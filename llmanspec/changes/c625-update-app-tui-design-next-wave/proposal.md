---
change_id: c625-update-app-tui-design-next-wave
title: "设计闸：下一波 TUI 形状（DESIGN + playground）"
status: purpose-draft
priority: 625
depends_on: []
author: agent
track: C
wave: design-gate
---

# c625-update-app-tui-design-next-wave

## Why

轨 B MVP 已通；下一波实现（`/models`、树 power、真 `$EDITOR`、footer context%、abort 反馈）需要**先固定视觉与交互形状**，避免边写边猜。配置继续 YAML+JSON Schema，**不做** Settings/Plate 运行时改配置。

## Purpose（本 draft 已落地的设计面）

更新产品视觉 SSOT 与浏览器静图，作为后续 A/B change 的引用锚点。

## What Changes（设计面；实现属后续 change）

1. `src/app/tui/DESIGN.md`：Next wave 表；组件索引加入 models-picker。
2. 新增 [`design/models-picker.md`](../../../src/app/tui/design/models-picker.md)。
3. 更新 [`keybindings.md`](../../../src/app/tui/design/keybindings.md) / [`session-tree.md`](../../../src/app/tui/design/session-tree.md) / [`bash-mode.md`](../../../src/app/tui/design/bash-mode.md) / [`footer.md`](../../../src/app/tui/design/footer.md)。
4. playground：新增 **Models**、**Tree power** 槽；slash 示意改为 `/models`。

## Capabilities（promote 时）

- `app-tui-design-playground`（modify：槽位与示意）
- 文档级：不新增运行时 capability

## Design SSOT（实现 MUST 引用）

| 主题 | 路径 |
|---|---|
| 总览 / Next wave | `src/app/tui/DESIGN.md` |
| `/models` | `src/app/tui/design/models-picker.md` |
| 键位 | `src/app/tui/design/keybindings.md` |
| 树 | `src/app/tui/design/session-tree.md` |
| 静图 | `src/app/tui/design/playground/`（`just open-design-playground`） |

## Impact

- 仅 DESIGN / playground / 本提案链；**不改** Rust 产品行为（本 change）。

## Out of scope

- Settings / Plate；computer-use；实现 `/models` / 树 power（见 c630+）。

## Ethics

- risk_level: low
- prohibited_actions: 不在 playground 堆 host 接线；不把静图当运行时真值
- required_evidence: playground 槽可打开；后续 change proposal 引用上表路径

## Promote 前

- 补 `specs/` + `tasks.md`（若需合约：playground 槽存在性 / Agent 忽略约定已有 c555）。
- 或本 change 仅作设计闸归档：确认静图后 archive，实现走依赖它的 A/B change。
