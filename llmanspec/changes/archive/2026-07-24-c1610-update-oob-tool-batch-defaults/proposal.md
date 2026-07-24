---
change_id: c1610-update-oob-tool-batch-defaults
title: 开箱 tool 批并行 + 默认多 tool 提示（收敛临时配置面）
status: in-progress
priority: 1610
depends_on:
- c1545-add-tool-batch-execution-mode
- c1605-add-runtime-prompt-fragments
author: agent
branch: feat/c1605-c1610-oob-prompt-batch
base_sha: 7c5075601088d1bfe49a865c31b80aa7bfaa24bf
checkpointed: true
checkpoint_sha: 7c5075601088d1bfe49a865c31b80aa7bfaa24bf
---

# c1610-update-oob-tool-batch-defaults

## Discussion context（2026-07-24）

- `c1545` 调度器正确；FIFO 实验批墙钟 2s；实验 2 显示本端 End→Done≈0 → `c1615` 搁置。
- 实验 3（APPEND + 硬性同消息）：`3f9473d5` / `50783e31` / `b75339ca`（+烟雾 `ec1351b6`）**multi_hit 4/4**。
- 开箱 = `barrier_parallel` + `c1605` 注入同文案片段；去掉手贴 APPEND 临时态。

## Why

翻转产品默认 + 默认多-tool 提示，收敛「只靠仓库 yaml 体验并行」。

## Product intent

| 项 | 目标 |
|---|---|
| `tool_batch.mode` 默认 | `barrier_parallel` |
| 配置面 | 保留 `sequential` 逃生 |
| 默认提示 | 经 `c1605` 注入实验 3 验证过的多-tool 策略 |

## Decisions

1. 默认模式翻转；`ar27`/`rc26` 期望随之改。
2. 默认 fragment 文案 = 实验 3 APPEND 精华（英短句）。
3. 开发仓可删冗余 `tool_batch.mode` / `APPEND_SYSTEM.md`。

## Experiments

### 实验 1 — 慢 I/O 并行 ✅

`c3889dc5-…` / 烟雾 `95338a3a-…`：批墙钟 ~2.007s。

### 实验 3 — 多-tool 提示 ✅

| session | multi_hit |
|---|---|
| `ec1351b6-…` | ✅ 3 |
| `3f9473d5-…` | ✅ 3 |
| `50783e31-…` | ✅ 3 |
| `b75339ca-…` | ✅ 3 |

**4/4 = 100%**，无 fake_parallel。

### OOB 验收（无 APPEND，c1605 片段）✅

| session | multi_hit | notes |
|---|---|---|
| `0362b702-96b6-47c1-a197-aad5e354e963` | ✅ 3 | Langfuse：每轮 `llm.request` 恰 1×`<runtime_policy>`；3×`tool.execute` `barrier_index=0` / `barrier_parallel`；api=`openai-responses` |

## Non-Goals

`c1615`；MCP ParallelSafe；删 `sequential`。

## Status

**verify green → finalize** on `feat/c1605-c1610-oob-prompt-batch`。

## Ethics

- risk_level: medium
- required_evidence: BDD 默认并行；单测片段启停；显式 sequential 仍可用；OOB session `0362b702-…`
