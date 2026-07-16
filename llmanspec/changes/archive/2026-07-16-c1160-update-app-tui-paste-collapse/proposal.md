---
change_id: c1160-update-app-tui-paste-collapse
title: "产品 TUI：长粘贴折叠 [paste #N +lines] 与恢复追踪"
status: full
priority: 1160
apply_band: P2-system
depends_on: []
author: agent
track: R
wave: paste
domain: app-tui
---

# c1160-update-app-tui-paste-collapse

## Why

包 `Editor` 已实现长粘贴折叠为 `[paste #id +N lines]` / `[paste #id N chars]`，但 `get_expanded_text` 未按完整 marker 替换（会残留 ` +N lines]`），且产品 host 提交/steer/follow-up/历史/Ctrl+G 仍走 `get_text()`，会把占位符送给模型。

## Purpose

长文本粘贴在 editor 显示为折叠占位；`get_expanded_text` 恢复完整正文；产品发送路径（idle 提交、steer、follow-up、`add_to_history`、Ctrl+G 读出）MUST 使用展开文本，折叠 UI 与提交全文一致。

## What Changes

- 包：`Editor::get_expanded_text` 按完整 marker 正则替换（对齐 pi `expandPasteMarkers`）
- 产品：`UiRoot` / host 发送路径改用 `get_expanded_text`
- harness：折叠显示 + 提交/展开全文一致
- 原子 marker 光标/删除（pi `segmentWithMarkers`）本变更不做；记入 `PI_DELTAS` 若需

## Capabilities

- `package-tui-editor`（add ed09）
- `app-tui-input`（add ati34）

## Out of scope

- 图片粘贴（c1155）
- 改 PasteBurst 时间阈值
- paste marker 原子分段光标（另开 change）

## Ethics

- risk_level: low
- prohibited_actions: 折叠导致提交丢失正文
- required_evidence: 折叠 UI + 提交全文一致测
- escalation_policy: —

## Depends

- []
