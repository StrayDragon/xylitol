---
change_id: c1585-fix-queue-drain-one-at-a-time
title: steer/follow-up 默认一次性只 drain 一条进入下一 turn（对齐 pi）
status: ready
priority: 1585
depends_on: []
author: agent
branch: feat/c1585-queue-one-at-a-time
base_sha: 7c5075601088d1bfe49a865c31b80aa7bfaa24bf
checkpointed: true
checkpoint_sha: 7c5075601088d1bfe49a865c31b80aa7bfaa24bf
---

# c1585-fix-queue-drain-one-at-a-time

> **决策已全部锁定**。延后修复；正交 c1580。

## Why

多条 Steering 同 turn 进 context（图 1–2）；pi 一次一条（图 3–4）。根因：开箱 `QueueMode::All`。

## Decisions（已锁）

### D1. 开箱默认

`QueueMode` / `SteeringMode` 默认改为 **`OneAtATime`**——**steer 与 follow_up 均改**。
配置仍可显式 `all`。本 change **不做** 产品 UI 热切换 mode。

### D2. 合约

缺省 MUST one-at-a-time；入队 2 条 → 第一轮只注入 1 条，队列剩 1。`ar10` abort 语义不变。

## Open Questions

（已清空。）

## Related

- `agent-runtime`、`runtime-config`、`docs/architecture/插话续跑与中止.md`

## Ethics

- `ethics.risk_level`: medium（默认变更）
- `ethics.required_evidence`: 队列/ReAct 测 + 可选 TUI harness
