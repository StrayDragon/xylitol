---
change_id: c485-add-app-tui-vertical-slice
title: "app-tui 垂直切片：可聊一轮 E2E"
status: draft
priority: 485
depends_on:
  - "c465-add-app-tui-bridge"
  - "c475-add-app-tui-chrome"
  - "c480-add-app-tui-input"
author: agent
track: B
---

# c485-add-app-tui-vertical-slice

> **status: draft**（已从 purpose-draft 升格；待 `llman-sdd-apply`）

## Why

轨 B 依赖（bridge / chrome / input / trust / abort-resume）已接线并归档，但仍缺「垂直切片」验收门槛：没有把 submit→stream/tool→steer/abort→`/exit` 收成可归档合约，产品二进制也无 PTY 冒烟。c485 是 MVP 归档闸，不是从零功能开发。

## What Changes

1. **合成 harness（MUST）**：`ScriptedDriver`（或等价假 Driver）+ 薄编排覆盖 H1–H10（见 design）——pending→Driver 调用、Xy 回流、工具行、队列 chrome、abort 后再提交、`/exit`→`finish_inline`。
2. **产品 PTY smoke（MUST）**：`tests/tui_e2e` 增加产品 `xylitol` 路径——temp Fake model + `--trust` → 就绪 → 提交 → 见默认 `"Hello from fake provider"` → `/exit` 进程退出。不做 steer/工具 PTY；不做跨进程 Fake 脚本化（thread-local 限制）。
3. **文档**：`DESIGN.md` / `AGENTS.md` 下一刀改为 c485；刷新过时 Overview。
4. **非目标**：bash(c492)、compaction UI(c493)、活树、c575、Codex transcript、跨进程 Fake scenario 基建。

## Capabilities

- `app-tui-vertical-slice`：垂直切片验收合约（合成 + 产品 PTY smoke）

## Impact

- 触达：`src/app/tui/tests.rs`（或 helper）、`src/app/core/driver` 测试用 ScriptedDriver、`tests/tui_e2e/*`、少量文档。
- 风险：PTY 慢/`#[ignore]`；Fake 默认文案变更会碎 needle——钉死字符串或脚注文案。
