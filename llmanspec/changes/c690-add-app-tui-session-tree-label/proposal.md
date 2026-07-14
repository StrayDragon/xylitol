---
change_id: c690-add-app-tui-session-tree-label
title: "产品会话树：Shift+L/T annotation 持久化（对齐 pi）"
status: full
priority: 690
depends_on: ["c645-update-app-tui-session-tree-fork"]
author: agent
track: A
---

# c690-add-app-tui-session-tree-label

## Why

包与 demo 已有 Shift+L 编辑 / Shift+T 时间戳；产品未接线，双 Esc 树里 Shift+L 无反应。`append_label_change` / `Label` entry 闲置。

## Purpose

树开时 Shift+L 打开槽内 Label 编辑；Enter 提交经 Driver 写入 `Label` entry；空串清除 annotation。Shift+T 切换时间戳显示（本 change：本地 `just now` / 显隐，**不**扩 domain `labelTimestamp`）。

## What Changes

1. 产品 `tree_label_edit` 槽（对齐 demo Input）。
2. `Driver::append_entry_label(target_id, label)` → `append_session_entry(Label)`。
3. Esc：先取消 label 编辑 → 再清搜索 → 再关树。
4. harness：Shift+L → 提交 → 树行见 `[annotation]`；Shift+T toggle。

## Capabilities

- `app-tui-session-tree` / `app-tui-input`

## Out of scope

- domain labelTimestamp 落盘；Settings；c710 debug

## Ethics

- risk_level: low
- prohibited_actions: 产品 reach infra
- required_evidence: harness + `--strict`

## Depends

- **c645**（已归档）；解锁 **c705**
