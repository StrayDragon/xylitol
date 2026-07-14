---
change_id: c715-update-app-tui-qa-baseline
title: "产品 TUI 阶段 QA 护栏：rstest-bdd 升级 + bang 路径同构 + 输入 BDD"
status: full
priority: 715
depends_on: []
author: agent
track: QA
wave: stage-qa-s0
---

# c715-update-app-tui-qa-baseline

## Why

下一波重构（abort 竞态止血、共享 bang 环、HostSession 拆分）若没有可复现护栏，易出现「harness 绿、真机红」：生产 bang 走嵌套 `select!`，harness 常用不可 Esc 的 `run_pending_bash`，手写 bang Esc 测又成第三份逻辑。rstest-bdd 已是仓库 BDD SSOT，但停在 0.6.0-beta2，且无产品 TUI feature。

## Purpose

在**不改用户可观察生产语义**的前提下（或仅测同构重排），落地 S0 护栏：

1. 升级 `rstest-bdd` / `rstest-bdd-macros` → **0.6.0-beta3**（或同系列更新），核心 BDD 全绿。
2. 抽出 **共享 bang Esc 可测 helper**，折叠 harness 手写 HRS；生产 `mod.rs` 与 harness 经同一入口（为 c725 全合并铺路；本变更允许生产仍内联调用 helper，但禁止第三份 select）。
3. 新增 rstest-bdd feature 固定：abort+迟到 delta、bang Esc/第二 bang、overlay vs 树 Esc、steer/follow-up/Alt+Up。
4. 记录 BASE/ABS/HRS/RPB 清单于 `design.md`，作为后续 c720+ 的必绿闸。

## What Changes

1. `Cargo.toml`：rstest-bdd → beta3；修升级 break（若有）。
2. `src/app/tui`：共享 bang 泵 helper（如 `effects::run_bang_select_loop` 或等价）；harness HRS 改用 helper；`pump_host_driver` Esc 敏感路径不再依赖裸 `run_pending_bash`。
3. `tests/features/app-tui-*.feature` + BDD step（复用 `HostSession`/`ScriptedDriver`/`HostEvent`，**禁止**第二套副作用泵）。
4. `design.md`：Esc 状态机摘要 + harness 标签清单。

## Capabilities

- `app-tui-host`（ath9）
- `app-tui-input`（ati30）
- `test-bdd`（bt2）

## Out of scope

- 立刻 arm `suppress_xy` / 主环 biased（→ **c720**）
- 删除生产嵌套环、ath7 彻底合一（→ **c725**；本变更只消测试三份逻辑）
- HostSession / UiRoot 模块拆分、PendingOps、AGENTS kimi 化（→ **c730** / **c716**）
- Plate/Settings 活面板；改 A01/A02/D09

## Ethics

- risk_level: low
- prohibited_actions: 以「方便测试」静默改变 abort/bang 用户可见文案或队列语义；为 BDD reach `agent::`/`infra::`
- required_evidence: `cargo test --test bdd -- --test-threads=1`；相关 harness ABS/HRS；`llman sdd validate c715-… --strict`
- escalation_policy: beta3 升级导致核心 BDD 大面积失败时 STOP，先修升级或回退并报告

## Depends

- 无硬依赖（基于已归档轨 B / c670 等现状）
- 后续：`c720` depends_on 本 change

## 验证

- Agent：`just fmt`；相关 clippy；harness ABS/HRS/B 系列；全量 bdd；`--strict`
- 人类：可选手测 bang Esc（不替代 harness）
