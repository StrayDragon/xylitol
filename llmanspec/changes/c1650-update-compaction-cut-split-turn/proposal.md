---
change_id: c1650-update-compaction-cut-split-turn
title: 切点与 split-turn 双摘要对齐 pi
status: designed
priority: 1650
depends_on:
- c1630-update-compaction-reserve-formula
author: agent
branch: sdd/c1650-update-compaction-cut-split-turn
base_sha: 8d1e8ba37a96b2a4d0d8375c071628224dcedddc
checkpointed: false
---

# c1650-update-compaction-cut-split-turn

> **依赖**：c1630 已归档。可与 c1640 并行（c1640 已归档亦不挡）。
> **对照**：`../pi/.../compaction/compaction.ts` — `findCutPoint` / `prepareCompaction` / split-turn 双摘要；`TURN_PREFIX_SUMMARIZATION_PROMPT`。
> **设计**：见同目录 `design.md` / `tasks.md`。

## Why

xylitol 切点几乎只认 user → `is_split_turn` 半死；超长单轮会丢 turn 前缀。pi：assistant 等可切 + history/turn-prefix **双摘要合并**。

## What Changes

- **合法切点**对齐 pi：user / assistant / bashExecution / custom / branchSummary；**永不** toolResult。
- **Split-turn**：`is_split_turn` 时 history + turn-prefix 双摘要，合并格式锁定（见 design）。
- **`tokens_before`**：优先同源上下文估计，非仅 boundary `len/4`。
- **不做**：overflow、instructions、travel LLM 分支摘要、改 auto/force 挂点。

## Capabilities

`domain-compaction`

## Impact

- 超长单轮可在 assistant 处切并保留 turn 前缀摘要。
- 切点行为对「仅 user 边界」的旧假设测试需改。

## Ethics

- risk_level: medium
- prohibited_actions: 半套 split-turn；推翻 A01；实现 overflow/instructions
- required_evidence: 验收锚点单测/BDD
- refusal_contract: 不做 travel LLM 分支摘要
- escalation_policy: 角色映射歧义以 design 对照表为准
