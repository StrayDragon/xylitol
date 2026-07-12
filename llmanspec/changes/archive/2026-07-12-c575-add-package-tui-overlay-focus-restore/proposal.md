---
change_id: c575-add-package-tui-overlay-focus-restore
title: "package-tui-overlay：完整 focus-restore（eligible/blocked）状态机"
status: draft
priority: 575
depends_on: []
author: agent
track: P
---

# c575-add-package-tui-overlay-focus-restore

> **status: draft**（已从 purpose-draft 升格；待 `llman-sdd-apply`）

## Why

pi-tui Overlay 有 eligible / blocked / resume 完整 focus-restore；xylitol-tui 目前仅最小 `hide` / `focus` / `unfocus`（`PI_DELTAS` **D08**）。多 overlay 场景需要可预测的焦点恢复，否则临时 `set_focus` 偷焦点后无法 reclaim，或 nested hide 落到错误目标。

## What Changes

1. **FocusTarget**：`pre_focus` 记录 root 索引或 overlay id（非仅 root index）；hide 时 retarget 依赖链。
2. **Restore SM**：`inactive` / `eligible` / `blocked{blockedBy, resume}`；`set_focus` 默认 clear；内部 redirect 用 preserve。
3. **dispatch_event**：listener 之后、投递之前 reclaim（eligible / blocked resume）；focused **可见** overlay（含显式 focus 的 non-capturing）接收输入。
4. **API**：`focus()` 允许对可见 non-capturing；`unfocus(options?)` 支持显式 `target`；临时 `set_hidden(false)` 对 NC 仍不自动抢焦点。
5. **Harness 单测**（无真实 TTY）：覆盖 reclaim、unfocus、NC、nested retarget 核心矩阵。
6. **台账**：`PI_DELTAS` D08 延后 → 已落地（host `dispatch_event` 路径）。

## Capabilities

- `package-tui-engine`：修订 `pte02`/`pte03`；新增 `pte06` focus-restore MUST

## Out of scope

- 产品 `src/app/tui` 接线
- `OverlayOptions.visible(w,h)`（可 follow-up）
- stdin-buffer / Image / 改 D09 host 驱动决议
- Container 全树 focus walk

## Impact

- 触达：`packages/xylitol-tui/src/tui.rs`、`lib.rs`、harness 测、`PI_DELTAS.md`、包 `AGENTS.md`
- 风险：medium（焦点状态机易回归）；以 pi overlay-non-capturing 焦点套件为对照
