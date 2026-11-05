---
depends_on: []
title: 修复 MCP 欢迎行闪烁
branch: sdd/c3205-fix-mcp-welcome-flicker
base_branch: main
base_sha: 6c58c8f47f9aa3dd3dfd6237833d9ad7d31c6c04
---

## Why

新会话启动 MCP 准备期间，`loaded_resources` 可能短暂返回无 MCP 的未完成快照；UI 直接覆盖后欢迎卡 `mcp` 行与 status cue 在显示/消失间抖动。

## What Changes

- `set_loaded_resources` 对“上一份有 MCP、下一份无 MCP 且 bootstrap 未完成”的快照保留上一份 MCP 字段。
- bootstrap 完成且确实无 MCP 时仍允许隐藏。
- 覆盖欢迎行与 MCP pending cue 不再抖动。
