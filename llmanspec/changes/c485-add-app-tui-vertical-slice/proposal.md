---
change_id: c485-add-app-tui-vertical-slice
title: "app-tui 垂直切片：可聊一轮 E2E"
status: purpose-draft
priority: 485
depends_on: ["c470-add-app-tui-transcript", "c475-add-app-tui-chrome", "c480-add-app-tui-input"]
author: agent
track: B
---

# c485-add-app-tui-vertical-slice

> **status: purpose-draft**

## Purpose

端到端：TTY 启动 → trust 已决或跳过 → 提交 → 流式回复/工具/diff → steer/follow-up → abort → `/exit` restore。作为轨 B MVP 归档门槛。

## Out of scope

- session 树产品接线（c491）、bash（c492）、compaction UI（c493）除非已提前完成
