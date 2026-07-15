---
change_id: c1170-refactor-app-tui-god-modules
title: "重构：拆分 TUI host/root/effects/bridge God 文件"
status: full
priority: 1170
apply_band: P1-refactor
depends_on: []
author: agent
track: QA
wave: smell
domain: app-tui
---

# c1170-refactor-app-tui-god-modules

## Why

阶段 QA 坏味道：`host/mod.rs` / `layout/root.rs` ≈1160 行，`effects` ≈800，`bridge` ≈975。AGENTS 已标 ~1200 硬味；继续堆 slash/reload 会加剧。

## Purpose

在**不改用户可观察语义**的前提下拆模块：input 策略、session mount/apply、slash catalog、slot 输入、effects slash 族、bridge UiModel 下沉；单文件目标显著低于 800 行（硬味阈 ~1200）；harness/BDD 全绿。

## What Changes

- 续拆 `host/`：`input_policy` + `session_ops`（或等价）
- `layout/`：`slash_catalog` + `slot_input`（或等价）；`root` 变薄协调
- `effects/` 目录：按 slash 族切文件，**唯一** `drain_pending` 入口不变
- `bridge/`：`UiModel` 类型与方法迁出 `mod.rs`；handlers 族保持
- 禁止 reach 内部；不改 PI_DELTAS；不改 abort/队列语义
- delta：`app-tui-host`（modify ath12）

## Capabilities

- `app-tui-host`（modify）

## Out of scope

- 新 slash / 功能
- Plate/Settings/Choice 解冻
- c1175 slash SSOT

## Ethics

- risk_level: low
- prohibited_actions: 以重构名义改 abort/队列/bang Esc 语义；引入第二套 drain 泵
- required_evidence: harness + app-tui BDD 绿；拆后各 God 文件行数可测下降
- escalation_policy: 若需改公开 API 形状先升格 specs

## Depends

- []
