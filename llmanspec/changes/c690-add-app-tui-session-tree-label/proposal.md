---
change_id: c690-add-app-tui-session-tree-label
title: "产品会话树：Shift+L/T annotation 持久化（对齐 pi）"
status: purpose-draft
priority: 690
depends_on: ["c645-update-app-tui-session-tree-fork"]
author: agent
track: A
---

# c690-add-app-tui-session-tree-label

## Why

包与 demo 已有 Shift+L 编辑 / Shift+T 时间戳；产品未接线，`append_label_change` 闲置。labeled-only filter 与 pi bookmark 体验缺半边。

## Purpose

树开时 Shift+L 打开槽内 LabelInput；提交后经 Driver/store 写入 `Label` entry；重开树 annotation 仍在。Shift+T 切换时间戳显示（domain `labelTimestamp` 是否落盘在 promote 时拍板）。

## What Changes（实现时）

1. 产品 `on_label_edit` + LabelInput 槽（对齐 demo）。
2. Driver / protocol 暴露 `append_label_change`（或等价 seam）；**不** reach infra。
3. harness：编辑 → persist → 重挂树见 annotation；Shift+T toggle。
4. 升格 full 时补 `app-tui-session-tree` / `app-tui-input` delta。

## Capabilities（planned）

- `app-tui-session-tree` / `app-tui-input`（modify）
- 可能轻触 `agent-session-store` / `protocol`（label API）

## Design SSOT

- [`session-tree.md`](../../../src/app/tui/design/session-tree.md) § Annotation
- [`keybindings.md`](../../../src/app/tui/design/keybindings.md)
- pi `appendLabelChange` + `LabelInput`

## Out of scope

- Settings 持久化默认 filter；branch summary on travel（c695）；slash（c700）

## Ethics

- risk_level: low
- prohibited_actions: 产品 reach `infra::session`；改父 session JSONL 以外旁路写 label
- required_evidence: harness persist + remount；升格后 `--strict` 绿

## Depends

- **c645**（已归档）；可与 **c685** 树槽 Search/Help 并行；**c705** E2E 依赖本 change
