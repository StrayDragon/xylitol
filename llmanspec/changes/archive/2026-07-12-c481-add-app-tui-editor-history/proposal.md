---
change_id: c481-add-app-tui-editor-history
title: "app-tui-input：Editor ↑/↓ 切换本 session 发送历史"
status: ready
priority: 481
depends_on:
  - "c480-add-app-tui-input"
author: agent
track: B
---

# c481-add-app-tui-editor-history

## Why

包 `Editor` 已实现 pi 同形历史浏览（ed05：空编辑器或浏览中 ↑/↓），但产品 Host 从未 `add_to_history`，用户无法在同一 TUI session 里用方向键召回刚发过的 prompt / steer / follow-up。

## What Changes

1. **接线**：idle Enter 提交、busy Enter（steer）、Alt+Enter（follow-up）在清空 editor 前 MUST 调用 `Editor::add_to_history`（去空、跳过连续重复；上限沿用包 100）。
2. **键位文档**：`keybindings.md` / `editor.md` 写明 ↑/↓ 在首/末可视行时浏览发送历史（与包行为一致；补全 popup 打开时仍由 SelectList 吃箭头）。
3. **单测**：host harness 验证提交后 ↑ 回填上次文本。

## Capabilities

- `app-tui-input`：产品 editor 历史填充（ati13）

## Out of scope

- 跨 session 持久化历史文件
- 改包 `navigate_history` 算法（ed05 已归档）
- 会话树 travel / fork 预填（另轨）

## Impact

- 触达：`src/app/tui/{host,ui_root,tests}.rs` + design 文档；不改 `packages/xylitol-tui` 算法。
