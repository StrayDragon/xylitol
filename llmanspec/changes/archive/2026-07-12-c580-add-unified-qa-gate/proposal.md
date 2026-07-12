---
change_id: c580-add-unified-qa-gate
title: "统一 just qa 满闸验证入口 + qa-e2e 第 5 层"
status: full
priority: 580
depends_on: []
author: agent
track: meta
---

# c580-add-unified-qa-gate

## Why

仓库已有 `just qa`，但组成与「大而全基准」边界未成文：包侧 TUI harness（1–4）、DESIGN token 闸、prek、以及环境相关的 PTY/tmux E2E（第 5 层）分散在多个 recipe 与 skill 里。需要**单一日常入口**锁定 PR/本地验收，同时把真终端 E2E 明确为可选满闸，避免缺 tmux 时整闸误红。

## Purpose

约定并实现统一验证入口：

| Recipe | 范围 |
|---|---|
| **`just qa`**（`check` / `ci` 别名） | 日常满闸：fmt-check → lint → test → **test-tui** → doc-check → check-tui-tokens → prek |
| **`just qa-e2e`** | `qa` + `test-tui-e2e`（portable-pty + tmux；`#[ignore]`；需本机环境） |

E2E **MUST NOT** 进入默认 `qa`（对齐 `package-tui-testing` tt06）。

## What Changes

1. Capability **`test-qa-gate`**：定义 `qa` / `qa-e2e` 组成与禁入项。
2. 更新根 [`justfile`](justfile)：`qa` 显式依赖 `test-tui`；新增 `qa-e2e`。
3. 更新根 [`AGENTS.md`](AGENTS.md) 命令段；[`test-tui-harness`](.claude/skills/test-tui-harness/SKILL.md) 指针与 `_HANDOFF` 短索引。
4. **不改** E2E 实现本身（已有 `tests/tui_e2e/` + recipes）。

## Capabilities

- `test-qa-gate`（新建）

## Impact

- `justfile`、根 `AGENTS.md`、TUI harness skill、`_HANDOFF.md`
- 本地/CI：`just qa` 多跑一次显式 `cargo test -p xylitol-tui`（与 workspace test 可能重叠，换**可复述**的满闸清单）

## Out of scope

- 把 E2E 默认并入 `qa`
- 新建截图级视觉 diff / CI 安装 tmux 强制矩阵
- 升格轨 B c475/c480

## Review（提案自审 · 锁定）

| 决定 | 选择 |
|---|---|
| 默认闸 | `just qa` 不含 E2E |
| 满闸含 E2E | `just qa-e2e` |
| TUI 1–4 | 经 `test-tui` 显式进入 `qa` |
| 方法论 | 已有：`package-tui-testing` + `test-tui-harness`；本 change 只钉入口 |
