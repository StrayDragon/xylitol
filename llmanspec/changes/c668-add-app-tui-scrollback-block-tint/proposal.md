---
change_id: c668-add-app-tui-scrollback-block-tint
title: "产品 scrollback：块级 bg tint 形态（bang / user / tool）"
status: full
priority: 668
depends_on: []
author: agent
track: B
wave: interrupt
blocks: ["c669-add-app-tui-scrollback-async-tint"]
---

# c668-add-app-tui-scrollback-block-tint

## Why

`agent_demo` 与 DESIGN 已定：user 全行 `user-message-bg`、tool/diff 头 `tool-*-bg` 三态。产品 `scrollback.rs` 对 user/tool 已部分接线，但 bang（`!`/`!!`）仍是裸 `System`/`Error` 行——无块级 tint，也与 demo 的 Tool 块视觉不一致。Track A（树 power）可并行，本变更 **插队** 先把形态钉死，便于 c669 再叠流式/并发。

## Purpose

产品 live scrollback：bang / user / tool（及 diff 头）按 DESIGN token **自动铺全行 bg**；bang 取消态显示块内 `(cancelled)`（已与 agent `Aborted` 分流）。**本变更不做** chunk 流式上行与多源并发合流（→ c669）。

## What Changes

1. **块模型**：引入可着色块（建议 `UiEntry::Bash { command, status, output, … }` 或等价 `BlockTint`），bang 不再长期依赖无语义的 `System` 拼行。
2. **渲染**：`widgets/scrollback.rs` 对 bang 块套 `tool-pending-bg` / `tool-success-bg` / `tool-error-bg`（cancelled → error tint）；user / tool 既有路径复核；**每块前后各一空行**（对齐 agent_demo）。
3. **文案**：bang Esc → 块内 `(cancelled)`；agent Esc → 仍 `Aborted`。
4. **DESIGN**：修订 [`bash-mode.md`](../../../src/app/tui/design/bash-mode.md)——去掉「bash 仅用 fg、非 tool bg」旧意向，改为块 tint MUST。
5. **Harness**：断言 bang 成功/失败/cancelled 渲染含 tint 痕迹；user/tool 回归；块间隙。

## Capabilities

- `app-tui-chrome`（modify：scrollback 渲染）
- `app-tui-bridge`（modify：`UiEntry` 块形态）
- 文档：`src/app/tui/design/bash-mode.md` / 必要时 `expandable.md`

## Design SSOT

- [`DESIGN.md`](../../../src/app/tui/DESIGN.md) § tool-*-bg / user-message-bg
- [`bash-mode.md`](../../../src/app/tui/design/bash-mode.md)（本变更修订）
- [`expandable.md`](../../../src/app/tui/design/expandable.md) § 工具块背景三态
- 形态参考：`packages/xylitol-tui/examples/agent_demo.rs`（`paint_tool_bg`）

## Impact

- `src/app/tui/bridge` · `widgets/scrollback.rs` · bang 上行路径（`commands` / `host`）
- **不**改 `Driver::execute_bash` 合约；**不**改包 `Palette` 色值（只消费既有 token）

## Out of scope

- bash `on_chunk` 流式上行与 pending 中途刷新（**c669**）
- 键盘 / bash future / agent 流三路并发合流设计（**c669**）
- Track A：models / 树 filter·fold·fork / `$EDITOR` / footer %

## Ethics

- risk_level: low
- prohibited_actions: 在 `packages/xylitol-tui` 塞产品 `UiEntry`；另起平行色板
- required_evidence: harness tint 断言；`just check-tui-tokens` 仍绿
- escalation_policy: 若要新增 token（非复用 tool-*-bg）须先改 DESIGN + sync

## Depends / 插队

- **depends_on: []**（c665 abort UI 已归档；本变更可独立 promote/apply）
- **插队**：priority 668，压过 Track A（c630–c655）；**blocks c669**
- 与树 power **无硬依赖**；可与 A 并行起草，实现优先本轨
