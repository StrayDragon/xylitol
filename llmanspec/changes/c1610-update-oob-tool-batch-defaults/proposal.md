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

## Discussion context

见 [`../DEFERRED-responses-tool-batch-CONTEXT.md`](../DEFERRED-responses-tool-batch-CONTEXT.md) 与 [`experiments.md`](./experiments.md)。

ROI 重估：调度器在模型配合时正确；开箱应用靠默认并行 + 默认提示，而不是长期依赖开发 yaml。

## Why

`c1545` 把 `barrier_parallel` 做成**试验档**、默认 `sequential`，开发仓靠 `.xylitol/config.yaml` 临时打开——与「pi 重写、开箱即用合适特性、少维护多面配置」方向相反。

实测：调度器在「同消息多 ParallelSafe」时正确；模型单 tool/turn 时 ROI≈0。默认 system prompt 几乎无「同消息并行 tool」约束（`default_prompt_base` 仅列出工具一行）。

## Product intent（终态开箱）

| 项 | 目标 |
|---|---|
| `tool_batch.mode` 产品默认 | **`barrier_parallel`**（写/bash/mcp 仍屏障） |
| 配置面 | 保留显式 `sequential` 逃生；**不**再需要开发剖面才能体验并行 |
| 默认提示 | 经 `c1605` 注入短片段：独立只读应同消息多 tool；禁止假装并行却单发 |
| 临时态 | 收敛「只靠仓库 yaml 开试验档」 |

## Decisions（意向）

1. 默认模式翻转 + 文档/architecture 更新；BDD 默认期望随之改（今日「默认 sequential」场景要翻）。
2. 默认 fragment 文案（草案，promote 时定稿）：强调同 assistant 消息发出全部独立 read/grep/find/ls；有写依赖则先读齐再写；勿叙事「并行」却单 tool。
3. TUI 策略切换（后置）：只改 Session mode → fragment 自动换（依赖覆盖盘 + `c1605`）；本 change 可不含 UI。
4. **不**为「提示强度」再加配置旋钮。

## Experiments（可先于 promote）

- **实验 1**：慢 I/O 并行证明（固定 prompt + Langfuse 清单）— 验证调度 ROI 上界。
- **实验 3**：仅注入多 tool 片段（可先手写 APPEND_SYSTEM）测 Ornith 同消息多 tool 命中率 — 验证提示 ROI，再固化进默认 fragment。

## What Changes（promote 后）

- 代码默认 `XyBatchMode::BarrierParallel`（或等价配置默认）
- 默认 fragment 内容 + 单测
- specs：`ar`/`rc` 默认期望；开发 yaml 可删掉仅用于开并行的段落
- ROI 说明进 architecture（批内加速；不替代预填治理）

## Non-Goals

- 流中抢跑（`c1615`）
- MCP 改 ParallelSafe
- 删 `sequential` 模式

## Status

**purpose-draft** — 建议：实验 3 命中率可接受后再翻转默认；机制先 `c1605`。

## Ethics

- risk_level: medium（默认并行改变写序窗口时序；屏障规则须保持）
- required_evidence: BDD 屏障窗仍绿；实验 1/3 记录
