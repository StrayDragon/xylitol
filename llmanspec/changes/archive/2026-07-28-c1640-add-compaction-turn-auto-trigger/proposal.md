---
change_id: c1640-add-compaction-turn-auto-trigger
title: Turn 后自动 compaction 与手动 force 路径（对齐 pi）
status: designed
priority: 1640
depends_on:
- c1630-update-compaction-reserve-formula
author: agent
branch: sdd/c1640-add-compaction-turn-auto-trigger
base_sha: 8d1e8ba37a96b2a4d0d8375c071628224dcedddc
checkpointed: true
checkpoint_sha: 8d1e8ba37a96b2a4d0d8375c071628224dcedddc
---

# c1640-add-compaction-turn-auto-trigger

> **依赖**：c1630 已归档（reserve 公式 + 无 `compaction_threshold`）。
> **对照**：`../pi/packages/coding-agent/src/core/agent-session.ts` — `compact()` / `_checkCompaction`（仅 threshold 支路；overflow 属 c1660）。
> **设计**：见同目录 `design.md` / `tasks.md`。

## Why

c1630 只换触发尺子。今日 `XyDriver::compact()` → `maybe_auto_compact`（先过闸），且 **turn 结束后无人调用**。pi：回合后 `_checkCompaction`；手动 `compact()` **强制**摘要。须补接线并拆开 auto / force。

## What Changes

- **Auto**：assistant 回合落定后（非 abort）在 agent/session 层跑 reserve 判定；发 `CompactionStart`/`End`（reason 含 `threshold`）。
- **Force**：`XyDriver::compact` / `/session-compact` / REST → `orchestrator.compact`；不过闸；prepare 失败返回 pi 同构错误串。
- **Stale**：compaction 边界前的 assistant/usage MUST NOT 再触发 threshold auto。
- **不做**：overflow、split-turn、instructions、extension 替换。

## Capabilities

`domain-compaction` · `agent-runtime` · `app-tui-commands` ·（driver / REST 经既有 dispatch）

## Impact

- 破坏性：未超闸时手动也会尝试压（对齐 pi）。
- 文档：architecture「Turn 后自动」归档后标落地。

## Ethics

- risk_level: medium
- prohibited_actions: 只改文档不接线；manual 继续 maybe；实现 overflow/split-turn/instructions
- required_evidence: 验收锚点单测或 BDD；Driver force 一处
- refusal_contract: 不实现 extension 自定义 compaction
- escalation_policy: 与 steer/follow-up 队列顺序不清时先对齐 design
