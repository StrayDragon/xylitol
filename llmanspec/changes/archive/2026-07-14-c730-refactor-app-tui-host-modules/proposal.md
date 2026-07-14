---
change_id: c730-refactor-app-tui-host-modules
title: "HostSession/UiRoot 下沉拆分、PendingOps 与 busy-bang 策略"
status: full
priority: 730
depends_on: ["c725-refactor-app-tui-shared-bang-loop"]
author: agent
track: QA
wave: stage-qa-s3
---

# c730-refactor-app-tui-host-modules

## Why

`host.rs` / `layout/root.rs` 近千行；下一波功能会继续堆。c716 文档地图需要代码边界兑现。另：agent busy 时 `!cmd` Enter 当前会当 steer 字面量（ati20 只硬拒 bash_active），属脚枪。

## Purpose

1. `PendingOps` 收拢 pending 标志；拆 host / UiRoot 子模块（协调者保持薄）。
2. slash 解析继续长在 `commands/`（可多文件）。
3. `get_messages` 失败 MUST 提示，禁止静默空 transcript。
4. agent busy 时 bang 前缀 Enter **硬拒绝**（ati32）。

## What Changes

1. 结构性拆分 + PendingOps；`drain_pending` / `step` 入口不变。
2. effects travel/fork：表面错误。
3. `try_busy_input`：agent busy 时 bang 前缀拒 steer。
4. harness/BDD 覆盖 ati32。

## Capabilities

- `app-tui-host`（ath12、ath13）
- `app-tui-input`（ati32）
- `app-tui-commands`（atm7）

## Out of scope

- 单 UiModel 所有权（future）；Plate/Settings 激活；permission popup；改 A01/A02

## Ethics

- risk_level: medium
- prohibited_actions: 拆分时引入第二套 drain；widgets 调 Driver；静默吞 get_messages 错
- required_evidence: 结构审计 + ati32 测 + BASE 绿

## Depends

- **c725** archive 后；建议 **c716** 已落地（文档地图），非硬 depends_on
