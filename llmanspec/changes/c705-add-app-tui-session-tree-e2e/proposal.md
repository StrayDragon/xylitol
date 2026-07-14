---
change_id: c705-add-app-tui-session-tree-e2e
title: "产品会话树 PTY/tmux E2E（强制闸）"
status: purpose-draft
priority: 705
depends_on:
  - "c685-update-app-tui-session-tree-slot-help"
  - "c690-add-app-tui-session-tree-label"
author: agent
track: A
---

# c705-add-app-tui-session-tree-e2e

## Why

harness 已覆盖 filter/fold/fork/travel 单元路径，但真终端协议、键序与 thinking 分块重建仍易漂移。对齐 pi 的体验验收 **MUST** 有可重复的第 5 层 E2E（非可选）。

## Purpose

在 `just qa-e2e` / `test-tui-e2e` 路径增加产品会话树场景：双 Esc 开树 → filter / 树槽 Search·Help 可见 → travel → Shift+F fork →（label 若已落地）Shift+L → transcript 按 v5 parts 重建 thinking/text。Fake 或脚本化 provider 优先；真 qwen 作补充笔记而非 CI 硬依赖。

## Verification

- **Agent 必跑**：落地后 `just test-tui-e2e-pty`（至少产品树相关 case）；缺 tmux 时注明并用 `-pty`。
- **人类确认**：同一 Fake 产品路径走一遍观感；只确认是否像 pi，修 bug 仍交 Agent。

## What Changes（实现时）

1. `tests/tui_e2e`（或等价）产品树场景；`#[ignore]` 惯例与现有 E2E 一致，由 `qa-e2e` 拉取。
2. 覆盖至少：开树、cycle/filter 状态行、Enter travel、Shift+F 新 session、thinking 与 assistant 分块。
3. 文档：何时跑 `just qa-e2e`；本机依赖（tmux/pty）。
4. 升格 full 时 capability：`test-qa-gate` 和/或 `app-tui-session-tree` 场景引用。

## Capabilities（planned）

- `test-qa-gate`（modify：会话树 E2E MUST）
- 可能 `app-tui-session-tree`（scenario 指针）

## Out of scope

- CI 强制装 tmux 全矩阵（沿用 c580：日常 `qa` 不含第 5 层；**发布/对齐验收 MUST 跑 `qa-e2e`**）
- Settings / branch-summary（c695）首版可不挡本闸

## Ethics

- risk_level: medium（真 PTY）
- prohibited_actions: 把需密钥的真云调用写进默认 CI `qa`；在 E2E 里写用户 home 外任意路径
- required_evidence: `just qa-e2e` 本机绿或明确 skip 原因；场景清单与 c685/c690 行为一一对应

## Depends

- **c685** + **c690**（未归档不可 apply）；fork/filter/fold/wire 已在更早归档 change
