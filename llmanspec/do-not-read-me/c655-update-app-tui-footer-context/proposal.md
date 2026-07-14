---
change_id: c655-update-app-tui-footer-context
title: "Footer：显示 used N tokens（原 context%）"
status: paused
priority: 655
depends_on: ["c625-update-app-tui-design-next-wave"]
author: agent
track: A
paused_reason: "token 用量展示依赖启发式/厂商 usage，估算可信度不足；延后至有可信数据源再 full/apply"
paused_date: 2026-07-14
paused_location: llmanspec/do-not-read-me/c655-update-app-tui-footer-context
---

# c655-update-app-tui-footer-context

> **⏸️ PAUSED（2026-07-14）** — 探索后认定「footer 展示已用 token / context%」在当前数据可信度下不宜落地。
> 本目录已移出 `llmanspec/changes/`，供人工核对：见 `llmanspec/do-not-read-me/c655-update-app-tui-footer-context/`。
> 恢复时：移回 `changes/`、更新 `status`、按 `design.md` 调研结论重新 full（specs+tasks）再 apply。
> 调研 SSOT：同目录 [`design.md`](./design.md)。

## Why

用户想知道当前会话（尤其 session tree travel 换叶后）上下文「用了多少」。DESIGN / c625 曾预留 footer `· context%`。

## Purpose（探索修订，未落地）

讨论中从 `· context%` 修订为更轻的 `· used <friendly> tokens`（不展示窗口总量/百分比；允许估计显示）。**因估算不准风险，整需求延后。**

## What Changes（实现时 — 仅备忘）

1. 经 Driver 只读 API 取当前 leaf 路径用量；映射到 footer。
2. 窄宽截断遵循 `src/app/tui/design/footer.md`（恢复前须先改 DESIGN 措辞）。
3. harness：空会话 / 有消息 / travel 换叶后数字变化。

## Capabilities

- `app-tui-chrome`（modify footer；曾拟新 `atc13`）
- Driver 只读状态（不 reach infra）

## Design SSOT

- [`design.md`](./design.md)（本 change 调研与延后理由）
- [`footer.md`](../../../src/app/tui/design/footer.md)（仍写 c655 / context%；恢复前同步改）
- [`DESIGN.md`](../../../src/app/tui/DESIGN.md) Next wave

## Impact

- 未改代码；恢复后：footer 渲染 + Driver 只读 seam

## Out of scope（延后期间全部）

- footer token / context% 产品展示
- 引入 tiktoken / 厂商精确 tokenizer 包
- 费用、↑↓ 分项、cache 命中率（pi 全量 footer）

## Ethics

- risk_level: low（展示）但 **product honesty medium**（假精确百分比/用量误导）
- prohibited_actions（恢复前）: 在无可信数据源时把启发式数字当「真 token」默认展示
- required_evidence（恢复时）: 明确数据源契约 + harness（含 travel）

## Depends

- **c625**（已归档）；与 c630 松耦合
