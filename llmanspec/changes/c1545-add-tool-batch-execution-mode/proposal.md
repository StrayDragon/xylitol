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

## Decisions（已收敛）

### D1. 权威来源 = 配置

运行时真源是 **配置 / Session API**（可动态改，turn 边界生效）。产品面暂不暴露 slash/UI。

实现可用 trait / 装配表填「并行安全」默认名单，但 **不以 trait 为用户权威**。

### D2. 默认对齐 pi：`parallel`

开箱默认与 pi 一致：`tool_execution = parallel`（纠正早前「先默认 sequential」意向）。

另保留 `sequential`：整批严格串行（保守档）。

### D3. xylitol 的 parallel = **两波调度**（非裸 `join_all` 全员并发）

同批例如 `read a` + `bash` + `read b`：

```text
Wave 1 — 并行安全工具（默认：read / find / grep，可配置扩展）
  全部并发执行，wait 齐结果（按源序写入 history）

Wave 2 — 其余工具（bash / edit / write / …）
  按 assistant 源序逐个串行
```

相对 pi 裸并行更安全；相对「整批因一个 Sequential 塌缩」更能保住纯读并发。

**不做**（本 change）：源序交错调度；「仅连续纯读前缀」严格源序并行。

### D4. 写安全 / 文件队列

**本 change 不做** per-path `FileMutationQueue`。edit/write 走 Wave 2 串行即可。

### D5. 事件与结果序

- `ToolExecutionEnd`：可按完成序发出（并行波内自然交错）
- history / 发往模型的 toolResult：**按 assistant 源序**

### D6. 与 Session / 类型默认不一致

Apply 时统一：Session 默认 `Parallel`（与类型默认、`D2` 一致）；去掉「Session=Sequential 而 enum default=Parallel」分裂。

## What Changes（意向）

- 接线 ReAct 批次调度：`sequential` | `parallel`（两波）
- 配置 / `set_tool_mode`（已有雏形）为权威；并行安全工具名单可配置或装配默认
- 文档化合成规则；新工具经配置/名单进入并行波，不改 ReAct 散落 if
- 暂不产品暴露动态切换 UI

## Non-Goals

- 文件变更队列（D4）
- 插件市场 / Extension Host
- 与 `c1540` 强制同发（已归档，正交）

## Open Questions

（已清空 — 见 Decisions。）

## Related

- `c1540` timeout（已归档，正交）
- `ar3` 已允许并行或串行
- pi：`agent-loop.ts`（默认 parallel）；本 change **刻意**用两波代替裸全并发

## Promote Gate

`purpose-draft`。决策已收敛；正式 propose 时补 design/tasks/live specs 并 attach。
