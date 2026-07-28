---
change_id: c1720-add-otel-turn-terminal-status
title: agent.turn 终态进 OTEL/Langfuse（abort vs ok）
status: designed
priority: 1720
depends_on: []
author: agent
branch: sdd/c1720-add-otel-turn-terminal-status
base_sha: 785db08a3bb45d7c7f7c8605587c17a7ea58b0aa
checkpointed: true
checkpoint_sha: 785db08a3bb45d7c7f7c8605587c17a7ea58b0aa
---

# c1720-add-otel-turn-terminal-status

> **排期**：apply 在 c1710 之后；**依赖边**：无（与 compact 正交）。
> **对照**：既有 otel17（`llm.request` abort）；c1700 `agent.compaction`。

## Why

用户 Esc、上游中断、流 drop 等会结束一轮，但 `agent.turn` 常无诚实终态属性；Langfuse 难以过滤「被取消的轮」vs「正常结束」。generation 层 abort（otel17）不够：turn 根仍像成功跑完。

## What Changes

- **Turn 终态**：低频观测激活时，`agent.turn` 结束 MUST 暴露可过滤终态：至少 **ok** 与 **aborted**（用户 cancel / 等价 abort 令牌导致的提前结束）。
- **属性**：失败/中止时 `langfuse.observation.level=ERROR`（或等价）+ `langfuse.observation.status_message`（如 `aborted`）；正常完成 MUST NOT 标 ERROR。
- **分类边界（首版）**：user-abort vs ok；模型/工具 **error** 类可标 `error` 若接线成本低，否则明确列为 follow-up（本 change 不阻塞）。
- **不做**：改 wire `XyEvent` 形状；默认甩敏感载荷；新建第二 exporter；子进程出站。

## Capabilities

`infra-otel`（主）· `infra-observability`（ipt4 对齐）

## Impact

- Langfuse 过程树可按 turn 终态筛选取消轮次。
- 与 otel17 generation abort 并存、不互相替代。

## Ethics

- risk_level: low
- prohibited_actions: 默认导出完整用户输入到 status；tracing 双栈；伪造 usage
- required_evidence: CollectingReporter（abort → ERROR/aborted；ok → 无 ERROR）；关闸 noop
- refusal_contract: 不做完整 cancel 原因枚举表若无代码真源
- escalation_policy: error vs abort 难分时先 abort/ok，另开 follow-up
