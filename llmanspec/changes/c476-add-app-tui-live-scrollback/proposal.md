---
change_id: c476-add-app-tui-live-scrollback
title: "app-tui-transcript：live scrollback 富渲染对齐 agent_demo"
status: ready
priority: 476
depends_on: ["c465-add-app-tui-bridge", "c475-add-app-tui-chrome"]
author: agent
track: B
---

# c476-add-app-tui-live-scrollback

## Why

c475 只接了 chrome 骨架；产品 transcript 仍是纯文本占位（`user:` / `(empty — submit…)`），对比 `just demo-tui` 的 Markdown / 工具色块 / Diff / Loader，观感落差大。`app-tui-transcript` 已有 att3–att8 合约，产品面尚未兑现。

## What Changes

1. `UiRoot` live 区按 `UiModel` 条目复用包组件渲染（对齐 demo 形态，非 Codex TranscriptView）：
   - 用户 / 助手：Markdown（`Palette::dark().markdown_theme()`）+ glyph / 可选 user-message-bg
   - thinking / tool：Expandable（折叠摘要 + `(Ctrl+T)` / `(Alt+E)` 旁注）；tool 行 `tool-*-bg`
   - edit diff：包 `Diff`；仅 header tint（att4）
2. busy status：短文案 + 可选 Loader/braille（仍 ≤1 行；Working 不进 footer）
3. Editor：对齐 demo 的 padding / muted 边框（已有 chrome 上补齐）
4. 单测：条目类型 → 渲染含 Markdown/Diff/bg 特征；idle 仍 0 status 行

## Capabilities

- `app-tui-transcript`

## Impact

- `src/app/tui/ui_root.rs`（及薄 `scrollback` 模块若需要）
- 不改 bridge 事件语义；可扩展 `UiEntry` 展示字段

## Out of scope

- Codex 式 TranscriptView（c470 paused）
- c491 活树 / 真 travel
- c480 slash / steer 键位（本变更可先接 Alt+E / Ctrl+T 仅作用于展开态）
- c490 trust ChoicePrompt（并行）
- demo Command plate / `/theme` 产品化

## Notes

- 形态 SSOT：`agent_demo` + `design/{markdown,diff-block,expandable,status}.md`
- 与 c485 垂直切片：本变更为「看起来像 demo」的前置；c485 仍管端到端可聊一轮
