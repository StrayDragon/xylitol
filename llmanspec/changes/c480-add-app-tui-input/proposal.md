---
change_id: c480-add-app-tui-input
title: "app-tui-input：Editor、slash、steer/follow-up 键位"
status: purpose-draft
priority: 480
depends_on: ["c460-add-app-tui-host", "c461-expose-steer-followup-seam", "c455-add-package-tui-input-listener"]
author: agent
track: B
---

# c480-add-app-tui-input

> **status: purpose-draft**（依赖 c465 事件合流后再接线键位更稳）

## Purpose

Editor 操作区；`/exit` `/model`；CompletionSource（包注册表已支持 `/` `@` `$` 扩展点）；键位：
- 流中 Enter → steer
- Alt+Enter → follow-up
- Esc → abort（流中）/ 关选择器
- Ctrl+C → 有内容清缓冲；空则退出
- 双 Esc → **c491 stub** 会话树（假树槽；真活树另 change，勿在 stub 上扩展）

## Out of scope

- 产品 palette/settings 全量（demo Command plate 已有；产品可极简 slash 先行）
- bash `!`（c492）
- 真 session 树 / Driver travel
