---
change_id: c494-refactor-app-tui-editor-slot
title: "产品 TUI：EditorSlot 槽位机 + 共享 effect 泵"
status: draft
priority: 494
depends_on:
  - "c465-add-app-tui-bridge"
  - "c475-add-app-tui-chrome"
  - "c480-add-app-tui-input"
  - "c485-add-app-tui-vertical-slice"
  - "c491-add-app-tui-session-tree"
author: agent
track: B
---

# c494-refactor-app-tui-editor-slot

## Why

`layout/` + `widgets/` 已落地，但 editor 区仍是 `tree_open: bool` 硬分支；`run_host_loop` 与 `harness::pump_host_driver` 双份副作用泵；扩展板/设置/Ask 只能继续堆 flag。应对齐 `agent_demo` 的 **editor 槽替换**结构能力（非像素），否则后续能力接线成本持续上升。

## What Changes

1. **`EditorSlot` 槽位机**（`layout/slots.rs`）：至少 `Editor | Tree(stub)`；MAY 含 `Plate` / `Settings` / `Choice`。互斥换槽、Esc 关槽还原 Editor；固定壳序不变（scrollback → queue strip → status → **slot** → footer）。
2. **`effects` + `commands`**：唯一 `drain_pending`（生产与 harness 共用）；slash/bang 解析收口。
3. **`bridge/handlers/`**：按事件族拆分，`apply_xy_event` 仍为唯一入口（保持 render 层不 match `XyEvent`）。
4. **对齐 demo 能力（本 change）**：槽机结构；Plate/Settings/Choice 可空壳接线；既有 steer/bash/fold/compaction 不回归。

## 非目标

- **c491 stub 冻结**：不扩活树 / filter / 真 Driver travel / fork UI。
- 产品路径不引入 `TUI::start()`。
- 不 rename capability id `app-tui-chrome`。
- 不做 capturing overlay 主交互；不与 `ReActAgent` rename（c585）同 PR。

## Capabilities

- `app-tui-input`：新增 `ati18` EditorSlot 合约
- `app-tui-host`：新增 `ath6` 共享 effect 泵

## Impact

- 触达：`src/app/tui/{mod,host,harness,layout,widgets,bridge}`；AGENTS 模块指针。
- 风险：Esc 优先级（abort vs 关槽 vs 双 Esc 开树）；双泵合并漏分支。
- 验收：H1–H9 全绿；假树仍仅双 Esc / Esc / Enter `travel → id`。
