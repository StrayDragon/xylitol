---
depends_on: []
blocks: []
title: 自动化引擎视未持久化会话为空分支
needs_specs_change: false
branch: sdd/c3348-fix-automation-unpersisted-session
base_branch: main
base_sha: d1ca28e85d52693ed8e708d07ff3d8b49a38d355
---

# 自动化引擎视未持久化会话为空分支

## Why

空闲 TUI 未跑过任何回合即关闭时,该会话在 store 中从未持久化;宿主 automation 引擎每秒 `tick` → `load_leaf_branch` → `NotFound`,宿主按错误每秒刷一条 `automation admit failed: session store: session not found` WARN,多个残留引擎叠加后刷屏。引擎 task 本身在 slot 驱逐(`evict_slot` abort)时正确终止,问题只在「未持久化窗口」的错误语义。

## What Changes

- `AutomationEngine::tick` 的 `load_leaf_branch` 返回 `XySessionStoreError::NotFound` 时视为**空分支**(无自动化事实 → 无动作),不再作为错误上抛;其余 store 错误语义不变。
- 宿主 `automation_loop` 的 WARN 随之消失(NotFound 不再进入错误分支)。
- 无合约变更(`agent-goal`/自动化行为面不变);`needs_specs_change: false`。

## Impact

- crates:`xylitol-runtime`(`automation.rs` tick + 单测:未持久化会话 tick 返回空动作)。
