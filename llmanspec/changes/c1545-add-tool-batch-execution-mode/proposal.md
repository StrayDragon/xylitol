---
change_id: c1545-add-tool-batch-execution-mode
title: 同 turn 工具批次可配置并行/顺序调度（对齐 pi，开闭可扩展）
status: purpose-draft
priority: 1545
depends_on: []
author: agent
---

# c1545-add-tool-batch-execution-mode

## Why

同一 assistant turn 可产生多个 tool call。xylitol 已有 `XyToolExecutionMode` 与 `XyTool::execution_mode()`，但 ReAct 将 `tool_mode` 丢弃为 `_tool_mode`，**固定顺序 `for` + await**。

合约 **已允许**并行：`agent-runtime` `ar3` 写明「可并行或串行」。缺口在实现与可配置权威来源，不在「要不要改 MUST」。

pi 默认 **parallel**：preflight 串行 → execute 并发；全局 sequential **或** 批次任一工具 sequential → **整批**串行。edit/write 写安全靠 **同路径文件队列**，而非把 bash/读工具整批拖死。

## 探索报告（批次执行 / 第 3 点）

### 现状（xylitol）

```text
MessageEnd → for tool in tool_calls:
  Start → await execute (+ select Update) → End → history
→ 全部完成后下一轮 LLM
```

- **顺序同步 await**（async 语境下的串行），不是 `join!` / 并行分发
- `SessionManager` 默认 `Sequential`；`XyToolExecutionMode` 枚举默认却是 `Parallel`（类型默认与 session 默认不一致）
- 仅 `edit` override 为 `Sequential`；read/grep/find/bash 走 trait 默认 Parallel（但未接线故无感）

### pi

| 项 | 行为 |
|---|---|
| 默认 | `toolExecution: "parallel"` |
| 批次规则 | `global_seq \|\| any(tool.executionMode==sequential)` → 整批串行；否则并行 execute |
| read vs bash | **无区别**（均可并行） |
| 写安全 | `withFileMutationQueue(path)` 同文件串行、跨文件仍并行 |
| 事件 | End 按完成序；toolResult 消息按 assistant **源序** |

### 配置 vs trait（Open Question 收敛方向）

**不钉死「只能 trait」。** 推荐混合（promote 前可再改）：

1. **可动态配置的默认模式**（会话 / settings / `set_tool_mode` 已有雏形）— 可在 **turn 边界**切换
2. **工具偏好** — 实现可以是 `XyTool::execution_mode()`，也可以是装配时注入的元数据（便于测试与动态替换）
3. **合成规则**对齐 pi：硬 Sequential 偏好可把整批打成串行
4. edit/write：**文件队列优于整批 Sequential**（避免 read+edit 被拖死）

当前顺序执行可作正确基线保留，直到本 change apply。

## What Changes（意向）

- 接线批次调度 + 文档化合成规则
- 配置面可动态改（turn 边界）
- 可选 per-path mutation queue
- 新工具开闭：声明偏好或服从配置，不改 ReAct 内核散落 if

## Non-Goals

- 本草案不改代码
- 不与 `c1540` 强制同发
- 不做插件市场

## Open Questions

1. 权威来源：仅配置 / 仅 trait / **混合（倾向）**？
2. 动态切换入口：settings / Session API / Driver / slash？产品是否暴露？
3. 写安全：文件队列 vs edit Sequential？
4. bash 默认 Parallel（跟 pi）还是 Sequential（更保守）？
5. Session 默认 Sequential vs 类型默认 Parallel — apply 时统一跟谁？

## Related

- `c1540` timeout（正交）
- `ar3` 已允许并行或串行
- pi：`agent-loop.ts` + `file-mutation-queue.ts`

## Promote Gate

`purpose-draft`。Open Questions 收敛后再正式 propose。
