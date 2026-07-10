---
change_id: c485-add-app-tui-vertical-slice
title: "app-tui 垂直切片：可聊一轮 E2E"
status: purpose-draft
priority: 485
depends_on: ["c475-add-app-tui-chrome", "c480-add-app-tui-input", "c465-add-app-tui-bridge"]
author: agent
track: B
---

# c485-add-app-tui-vertical-slice

> **status: purpose-draft**（轨 B MVP 归档门槛；依赖 c465 + c475 + c480）

## Purpose

端到端：TTY 启动 → trust 已决或跳过 → 提交 → 流式回复/工具 → steer/follow-up → abort → `/exit` restore。

## 依赖说明

- **不再**硬依赖 c470（Codex 式 TranscriptView 已搁置）。
- Live 输出以 bridge + 极简 scrollback 为准；会话树保持 **c491 stub**（真活树另 change，可与本切片并行提案）。

## Out of scope

- Codex 式 transcript 浏览器
- bash（c492）、compaction UI（c493）除非已提前完成
- Overlay 完整 focus-restore（c575，可选）
