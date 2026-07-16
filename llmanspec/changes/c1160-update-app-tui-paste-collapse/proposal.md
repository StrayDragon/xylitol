---
change_id: c1160-update-app-tui-paste-collapse
title: "产品 TUI：长粘贴折叠 [paste #N +lines] 与恢复追踪"
status: purpose-draft
priority: 1160
depends_on: []
author: agent
track: R
wave: paste
domain: app-tui
---

# c1160-update-app-tui-paste-collapse

## Why

包 `Editor` 已实现 `[paste #id +N lines]` / chars 折叠与展开恢复（对齐 pi）。需确认**产品 host 路径**完整启用、与提交/历史一致，并对照 `../pi` 补齐追踪缺口（若有）。

## Purpose

产品 TUI 长文本粘贴显示为 `[paste #N +lines]`（或 chars）占位；可展开/自动恢复原文；paste id 可追踪；提交给模型的是完整文本（或明确的展开语义）。参考 pi；以包实现为基线，产品只接线与回归。

## What Changes（升格 full 时）

- 审计产品 editor 是否走包 PasteBurst/collapse；补缺口
- harness：折叠显示、展开、提交全文
- 对照 pi 差异记入 `PI_DELTAS`（若有意）
- delta：`app-tui-input` · `package-tui-paste-burst` / editor

## Capabilities

- `app-tui-input`（modify）
- `package-tui-paste-burst`（modify 若需）
- `package-tui-editor`（modify 若需）

## Out of scope

- 图片粘贴（c1155）
- 改 PasteBurst 时间阈值（除非 bug）

## Ethics

- risk_level: low
- prohibited_actions: 折叠导致提交丢失正文
- required_evidence: 折叠 UI + 提交全文一致测
- escalation_policy: 包已完备则本 change 可缩为「产品回归 + 文档」quick

## Depends

- []
