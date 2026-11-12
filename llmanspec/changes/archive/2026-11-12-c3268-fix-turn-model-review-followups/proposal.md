---
depends_on: []
title: "Fix turn-model review follow-ups"
branch: sdd/c3268-fix-turn-model-review-followups
base_branch: main
base_sha: e552e0ff4271ab4e275157ddd5657c04f74f9129
needs_specs_change: true
---

# turn 模型复核收尾（c3268）

> 2026-11-12 c3266+c3267 验收 review 产出。1 CRITICAL + 4 WARNING 收口；goal 消息角色调研独立进行，不在本 change 范围。

## Why

c3267 验收发现：steer「settle 后到达视为一次提交（新 turn）」是落地 spec r3322 的 MUST 条款（也是用户 A 语义决策原文「followup/steerup 都是独立的单个提交模式」），但实现让 steer 永远走 steer 队列——空闲期到达与下次用户消息**合并进同一次模型调用**（测试名 joins_next_submission 自证违背），边界模糊在 steer 身上未死。另有三处测试缺口（任务勾选但无测试）与两处场景步骤文本的 run 残留。

## What Changes

- C-1：steer 双路径路由——turn 在途 → steer 队列（既有 A 语义注入当前 turn）；turn 不在途（settle 后 / 空闲）→ **follow_up FIFO**（视为一次提交，新 turn）。改写误导测试 joins_next_submission 为新语义断言。
- abort 语义对齐 r3322 通读：abort 取消当前 turn、清 steer；**FIFO 非空时立即弹出下一条开新 turn**（现实现直接结束流，FIFO 挂起到下次提交——与条款不符）。
- 补测 ×3：steer 在途注入当前 turn（runtime 级）；abort 后 FIFO 立即弹下一条；context_reminder 的 fork 祖先链保留（c3266 T11 漏项）。
- app-tui-input 场景步骤文本 run→turn ×2（「run 不再活跃」「run 收到 second」）+ BDD bindings 同步。

## Capabilities

- agent-runtime（steer 路由与 abort 条款实现补齐）；app-tui-input（场景措辞）。

## Impact

- 范围：crates/xylitol-runtime（capabilities steer / react 循环）、tests/bdd、app-tui-input spec。无 breaking（使实现符合已落地 spec）。

## 决策记录（2026-11-12，全部已决）

1. （已决）C-1 路由判据 = turn 是否在途（非流是否活跃——turn 间隙到达也须成新 turn）。
2. （已决）abort 后 FIFO 非空立即弹下一条（r3322 通读；用户改向意图针对当前 turn，排队提交仍应投递）。
3. （已决）goal 续跑消息角色的裁决待调研（DSH / Codex 对照）另行进行，不入本 change。
4. （已决）场景文本改动必须同步 bindings 并跑对应 BDD（步骤文本精确绑定）。
