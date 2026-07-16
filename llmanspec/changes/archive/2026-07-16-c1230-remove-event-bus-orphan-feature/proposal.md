---
change_id: c1230-remove-event-bus-orphan-feature
title: "移除 event-bus 孤儿 feature（行为已由单测覆盖）"
status: proposal
priority: 1230
apply_band: P3-core
depends_on:
  - c1220-migrate-domain-security-bdd
author: agent
track: R
wave: bdd-migration
---

# c1230-remove-event-bus-orphan-feature

## Why

`tests/features/event-bus.feature` 含 4 个场景，从未在 `tests/bdd.rs` 或 `src/` 任何
地方实现 step 或 `#[scenario]` 绑定——是纯孤儿。EventBus 行为已由 `src/infra/event/mod.rs`
的 7 个 `#[tokio::test]` 单测充分覆盖（emit/receive、unsubscribe、drop、clear 等）。

按根 AGENTS.md「测试分层」原则（已有 BDD 的路径单测只测边界；反之已有单测的底层不重复
BDD），保留这个孤儿 feature 是债务而非资产。

## What Changes

1. 删除 `tests/features/event-bus.feature`（4 个未实现场景）。
2. delta `test-bdd` modify r1：把过时的"77 个场景全通过"更新为"BDD 全量通过 + 孤儿 MUST
   移除"的通用合约（原 r1 硬编码数字 77 已失效，现 100 passed）。

## Capabilities

- `test-bdd` — modify r1（更新全量通过合约）

## Impact

- feature 文件 21 → 20。
- bdd 全量：100 passed，0 回归（孤儿本就无绑定，删除无影响）。
