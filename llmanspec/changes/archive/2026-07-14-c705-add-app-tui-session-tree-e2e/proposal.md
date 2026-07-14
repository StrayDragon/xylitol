---
change_id: c705-add-app-tui-session-tree-e2e
title: "产品会话树 PTY E2E（强制闸）"
status: full
priority: 705
depends_on:
  - "c685-update-app-tui-session-tree-slot-help"
  - "c690-add-app-tui-session-tree-label"
  - "c710-add-app-tui-debug-scenes"
author: agent
track: A
---

# c705-add-app-tui-session-tree-e2e

## Why

harness 已覆盖 filter/fold/fork/travel，但真终端键序与树槽 Search/Help/label 仍易漂移。对齐验收 MUST 有可重复第 5 层 PTY E2E。

## Purpose

在 `tests/tui_e2e` 增加产品 Fake 会话树场景（`#[ignore]`，由 `just test-tui-e2e-pty` / `qa-e2e` 拉取）：双 Esc 开树 → Search/Help 可见 →（可选）filter 状态 → Shift+L 编辑可见 → `/exit`。

## What Changes

1. `pty_product_fake_session_tree_*` 用例（至少开树 + Help/Search；label 一拍）。
2. `test-qa-gate` delta：会话树 PTY 场景 MUST 存在且经 `test-tui-e2e-pty` 可跑。
3. `app-tui-session-tree` scenario 指针（ast14）。
4. AGENTS / skill 一行指针（若缺）。

## Capabilities

- `test-qa-gate`（qg05）
- `app-tui-session-tree`（ast14）

## Out of scope

- CI 强制装 tmux；真云；c695 branch-summary；完整 fork 矩阵（首版可不挡）

## Ethics

- risk_level: medium（真 PTY）
- prohibited_actions: 真云密钥进默认 `qa`；写用户 home 外任意路径
- required_evidence: `just test-tui-e2e-pty` 本机绿

## Depends

- **c685** + **c690**（已归档）
