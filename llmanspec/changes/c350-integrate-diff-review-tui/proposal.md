---
change_id: c350-integrate-diff-review-tui
title: 将 diff_review 交互式 diff 审查样例集成进 TUI 作为一个 overlay/组件
status: proposed
priority: 350
depends_on:
  - c340-add-tui-inline-repl
author: agent
---

# c350-integrate-diff-review-tui

## Why

`src/app/tui/diff_review/`（CLI/types/mod，约 2.5k 行）是一个**交互式 diff 审查 demo**。它与 agent 主线**零契约**：不引用 `XyEvent`/`Driver`/`agent::*`，入口 `run_demo()` 经 `lib::run_review_demo()` 暴露但无任何调用方。write-tui skill 与 `src/app/tui/AGENTS.md` 都将其分类为真死代码，并明令「禁止在 diff_review 基础上扩展成 TUI——它与 agent 主线无契约，扩展等于在死码上盖楼」。

但用户（2026-07 指示）明确：**diff_review 是「之后集成进 TUI 的样例代码」，本轮保留不动，待需要时完成集成**。本变更是那个「需要时」的登记：把 diff_review 的交互式 diff 审查能力，作为一个 **TUI overlay/组件**正式集成进已落地的 TUI（c340），让它经 Driver + XyEvent 驱动，而非继续作为孤立项。

### 调研证据

- **当前状态**：diff_review 零 agent 契约，是真死码（audit-dead-code 第 3 节分类）。它的价值在「diff 审查的交互范式」（选中/导航/审批 UI），而非 agent 集成。
- **kimi-code 的对标**：kimi-code 的 TUI 有 `components/dialogs/`（diff 查看/审批等 overlay），这些 overlay 经 reverse-rpc 适配器与 SDK 回调对接——即「UI 组件 + seam 适配」模式，而非孤立 demo。
- **write-tui skill 约束**：集成时 MUST 经 Driver + XyEvent，MUST NOT 在 diff_review 死码上直接扩展。

## What Changes

（本变更为 draft，实施时再细化。核心方向：）

1. 评估 diff_review 的哪些交互范式（diff 渲染、选中、审批流）值得保留为 TUI 组件。
2. 将选中的范式重写/迁移为 `src/app/tui/components/` 下的正式组件（如 `components/diff_viewer.rs`），经 Driver + XyEvent 驱动（例如消费 `XyEvent::ToolExecutionEnd` 中的 diff 结果）。
3. 删除 diff_review 孤立 demo（`run_demo`/`lib::run_review_demo`），其能力已被正式组件取代。
4. 若 diff_review 经评估无独立价值（范式已被更好实现取代），则直接删除而非迁移。

## Capabilities

- `app-tui`（修改）：新增 diff 审查 overlay 组件。
- `diff-review`（修改）：从孤立 demo 升级为 TUI 集成组件（或删除）。

## Impact

- **受影响代码**：`src/app/tui/diff_review/`（删除或迁移）、`src/app/tui/components/`（新增）、`src/lib.rs`（移除 `run_review_demo`）。
- **受影响规范**：`app-tui`、`diff-review`。
- **风险**：低（c340 已落地 TUI 后，diff_review 集成是增量）。主要不确定性在 diff_review 范式的实际可复用度。

## 反降级护栏（防止本变更被降级为「保留 diff_review 孤立项 + 只加个入口」）

- [ ] 集成后的 diff 审查能力 MUST 经 Driver + XyEvent 驱动（MUST NOT 继续作为零契约的孤立项）。
- [ ] diff_review 孤立 demo（`run_demo`/`run_review_demo`）MUST 被删除——要么能力迁入正式组件，要么判定无价值直接删，**不得**「保留 demo + 另加入口」地两套并存。
- [ ] 若迁移，MUST NOT 在 diff_review 既有死码上直接扩展（write-tui skill 明令）。
