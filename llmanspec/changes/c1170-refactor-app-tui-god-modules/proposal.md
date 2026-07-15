---
change_id: c1170-refactor-app-tui-god-modules
title: "重构：拆分 TUI host/root/effects/bridge God 文件"
status: purpose-draft
priority: 1170
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

在**不改用户可观察语义**的前提下拆模块：pending 类型、bang 泵、completion 安装、bridge family handlers 下沉；单文件目标显著低于 1200 行；harness/BDD 全绿。

## What Changes（升格 full 时）

- 续拆 `host/` 子模块；`root` 槽/补全分离；`effects` 按 slash 族切文件
- 禁止 reach 内部；不改 PI_DELTAS
- delta：`app-tui-host`（modify，行为不变）

## Out of scope

- 新 slash / 功能
- Plate/Settings 解冻

## Ethics

- risk_level: low
- prohibited_actions: 以重构名义改 abort/队列语义
- required_evidence: harness + app-tui BDD 绿；行数下降可测
- escalation_policy: 若需改公开 API 形状先升格 specs

## Depends

- []
