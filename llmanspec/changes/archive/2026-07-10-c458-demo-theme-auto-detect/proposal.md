---
change_id: c458-demo-theme-auto-detect
title: "agent_demo：主题自动亮暗探测扩展点验证"
status: full
priority: 458
depends_on: []
author: agent
track: A
---

# c458-demo-theme-auto-detect

## Why

包已有 `terminal_colors` / OSC11；产品 MVP 固定暗色，但扩展点要在 demo 验证以免日后无法接入。

## Purpose

demo 可选启用 OSC11 / CSI 997 / COLORFGBG 探测切换 token 集；默认仍暗色；不进产品 MVP。

## What Changes

1. 包：`terminal_colors` 增补相对亮度、COLORFGBG 解析、以及「多源合成 scheme」纯函数（无真实 TTY）。
2. demo：默认 Dark；`XYLITOL_AGENT_DEMO_THEME_AUTO=1` 或 harness API 启用探测后，按探测结果切换 Light/Dark token 并在 chrome 可见。
3. harness：注入 OSC11 / COLORFGBG / 997 报告，断言 `theme_mode`。
4. 文档：`theme-tokens.md` 记录 demo 扩展点与产品默认暗色。

## Capabilities

- `package-tui-terminal-protocol`（颜色探测纯函数）
- `app-tui-chrome`（demo 可选 auto theme）

## Out of scope

- 产品默认自动切换
- 事件循环内真实 OSC 查询竞态处理（仅验证解析与切换形态）
