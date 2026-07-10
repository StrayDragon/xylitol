---
change_id: c458-demo-theme-auto-detect
title: "agent_demo：主题自动亮暗探测扩展点验证"
status: purpose-draft
priority: 458
depends_on: []
author: agent
track: A
---

# c458-demo-theme-auto-detect

> **status: purpose-draft**

## Why

包已有 `terminal_colors` / OSC11；产品 MVP 固定暗色，但扩展点要在 demo 验证以免日后无法接入。

## Purpose

demo 可选启用 OSC11/COLORFGBG 探测切换 token 集；默认仍暗色；不进产品 MVP。

## Out of scope

- 产品默认自动切换
