---
change_id: c685-update-app-tui-session-tree-slot-help
title: "产品会话树槽：动态 TreeHelp + Search 行 + cycleBackward"
status: full
priority: 685
depends_on: ["c645-update-app-tui-session-tree-fork"]
author: agent
track: A
---

# c685-update-app-tui-session-tree-slot-help

> 命名：本 change 说的是 **树槽 Search/Help（layout）**，不是浏览器 chrome。
> 历史 capability `app-tui-chrome` 合约 id **不改名**（语义=layout 壳，见 `src/app/tui/AGENTS.md`）。

## Why

产品树槽仍写死一行 `Up/Down Enter travel…`，不含 filter/fold/fork，也未跟 `KeybindingsManager`。pi 用 `TreeHelp` + `SearchLine` 动态出键；demo 有静态 hint。体验差在 **槽头提示**，不是缺能力。

## Purpose

树开时：Search 行（空=`Type to search`；有查询=`Search: …`）+ 由 keybindings 解析的 TreeHelp（move / page / branch / filters / cycle；label 键可显示但编辑属 c690）；Ctrl+Shift+O cycleBackward；刷新 vs-pi / keybindings 真值。

## What Changes

1. 产品 `render_editor_slot` Tree：去掉过时硬编码 hint；加 Search 行 + 动态 Help 行（宽窄换行对齐 pi `·` 分隔）。
2. Help 文案键位 MUST 经包 `KeybindingsManager`（或产品只读封装）取当前绑定，**MUST NOT** 写死 `Ctrl+T` 等字面除非解析失败回落。
3. 树开：Ctrl+Shift+O（或解析到的 `cycleBackward` 和弦）→ FilterMode 反向循环；更新 `ati22` / 新增键合约。
4. 同步 `session-tree.md` / `session-tree-vs-pi.md` / `keybindings.md`（含已归档 fork 真值）。
5. harness：开树帧含动态 hint 片段；cycleBackward；搜索行随查询变化。

## Capabilities

- `app-tui-session-tree`（modify：树槽 Search/Help）
- `app-tui-input`（modify：cycleBackward）

## Design SSOT

- [`design.md`](./design.md)
- pi `tree-selector.ts` `TreeHelp` / `SearchLine` / `filter.cycleBackward`
- [`session-tree.md`](../../../src/app/tui/design/session-tree.md)
- [`keybindings.md`](../../../src/app/tui/design/keybindings.md)

## Impact

- `src/app/tui/layout/root.rs`（及少量 theme/helper）；MAY 在 `packages/xylitol-tui` 加纯函数 `format_tree_help`（零产品依赖）
- **不**改 fork/travel/store；**不**实现 Shift+L 持久化（c690）

## Out of scope

- LabelInput / persist（c690）；branch summary（c695）；`/tree` `/fork`（c700）；PTY E2E（c705）；Settings

## Verification（自验 + 人辅）

见本 change [`design.md`](./design.md)「验证」与 `src/app/tui/AGENTS.md`「产品 UI 验证」。

## Ethics

- risk_level: low
- prohibited_actions: 把产品 FilterMode 硬编码进包；Settings 运行时改键为本 change 范围
- required_evidence: harness 树槽 Search/Help + cycleBackward；`--strict` 绿
- escalation_policy: 无

## Depends

- **c645**（已归档）；解锁 **c705**（与 c690 一起）
