---
change_id: c1610-update-oob-tool-batch-defaults
title: 开箱 tool 批并行 + 默认多 tool 提示（收敛临时配置面）
status: purpose-draft
priority: 1610
depends_on:
  - c1545-add-tool-batch-execution-mode
  - c1605-add-runtime-prompt-fragments
author: agent
---

# c1610-update-oob-tool-batch-defaults

## Discussion context（2026-07-24）

- `c1545` 调度器在「同消息多 ParallelSafe」时正确（含 FIFO 实验：三 read delay=2s → 批墙钟 2.001s）。
- 默认仍 `sequential`、靠开发 yaml 开试验档 = 临时态；产品要开箱合适特性、少配置面。
- 模型不配合时靠提示可改善（APPEND 烟雾：session `ec1351b6-…` 同消息 3×read）；默认 fragment 经 `c1605` 注入。
- 并行不解决预填大头；ROI = 批内执行加速。

## Why

翻转产品默认 + 默认多-tool 提示，收敛「只靠仓库 yaml 体验并行」。

## Product intent

| 项 | 目标 |
|---|---|
| `tool_batch.mode` 默认 | `barrier_parallel` |
| 配置面 | 保留 `sequential` 逃生；勿再靠开发剖面 |
| 默认提示 | 独立只读同消息多 tool；禁止假装并行却单发 |

## Decisions（意向）

1. 默认模式翻转 + BDD/文档。
2. 默认 fragment 文案 promote 时定稿。
3. TUI 策略切换后置（依赖覆盖盘 + `c1605`）。
4. 不为提示强度再加旋钮。

## Experiments（可先于 promote）

### 实验 1 — 慢 I/O 并行

前置：终端跑 `python3 scripts/xylitol_batch_slow_fifos.py --delay 2`；`tool_batch.mode=barrier_parallel`。

用户提示词见会话交付（同消息 3× read FIFO）。判据：批墙钟 ≈2s 非 ≈6s；Langfuse 同 `barrier_index`。

烟雾：`95338a3a-…` → 2.001s。

### 实验 3 — 多-tool 提示命中率

`.xylitol/APPEND_SYSTEM.md` 已含策略片段。固定 3 小文件同消息 read，N=10 计 `multi_hit` / `fake_parallel`。

烟雾：`ec1351b6-…` → toolCall 个数=3。

## Non-Goals

流中抢跑（`c1615`）；MCP ParallelSafe；删 `sequential`。

## Status

**purpose-draft** — 实验 3 命中率可接受后再翻转默认。

## Ethics

- risk_level: medium
- required_evidence: BDD 屏障仍绿；实验 1/3
