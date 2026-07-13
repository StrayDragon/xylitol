---
change_id: c590-update-test-qa-gate-scripts
title: "test-qa-gate：scripts/check_* 纳入 just qa 合约"
status: draft
priority: 590
depends_on:
  - "c580-add-unified-qa-gate"
author: agent
---

# c590-update-test-qa-gate-scripts

## Why

justfile 已落地 `check-scripts-wired` / `check-scripts`，但 `test-qa-gate` qg01 未写入合约，文档与 MUST 会漂移。

## What Changes

1. 修改 qg01：满闸 MUST 含 scripts/check_* 入闸校验与执行。
2. 新增 qg04：check_* 约定 vs cleanup_* 维护脚本。

## 非目标

- 不改 qa 其它步骤顺序语义；不把 cleanup_* 拉进 qa。

## Capabilities

- `test-qa-gate`
