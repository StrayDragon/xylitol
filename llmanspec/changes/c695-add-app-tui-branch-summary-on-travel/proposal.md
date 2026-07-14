---
change_id: c695-add-app-tui-branch-summary-on-travel
title: "产品会话树 travel：可选 branch summary（对齐 pi）"
status: purpose-draft
priority: 695
depends_on: ["c615-update-app-tui-live-session-tree"]
author: agent
track: A
---

# c695-add-app-tui-branch-summary-on-travel

## Why

pi 在 tree Enter travel 前可问 Summarize / custom prompt；xylitol 直接 travel。长分支回退时缺摘要落点。

## Purpose

travel 前可选 branch summary（可跳过）；取消可回到树；忙碌/abort 语义对齐既有 status。

## What Changes（实现时）

1. 对齐 pi：selector →（可选）summary choice → `navigateTree`/`travel` + summarize。
2. skip-prompt 偏好：仅当配置面开闸后再接（本仓 Settings 槽未开则默认每次问或默认 No）。
3. harness / 升格 delta：取消重开树；abort summarization。

## Capabilities（planned）

- `app-tui-session-tree` / 可能 `agent-compaction` 或 session travel API

## Out of scope

- Settings UI；树槽 Search/Help / label（c685 / c690）

## Ethics

- risk_level: medium（LLM summary 副作用）
- prohibited_actions: 无用户确认强制扣费摘要为唯一路径
- required_evidence: 可跳过路径 + 取消回树

## Depends

- **c615**（已归档）；建议在 **c690** 与 **c705** 主路径之后再 promote
