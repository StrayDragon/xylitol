---
change_id: c1545-add-tool-batch-execution-mode
title: 同 turn 工具批：默认同序；可配置写屏障并行扇出（异于 pi）
status: proposed
priority: 1545
depends_on: []
author: agent
branch: feat/c1545-add-tool-batch-execution-mode
base_sha: 7c5075601088d1bfe49a865c31b80aa7bfaa24bf
checkpointed: true
checkpoint_sha: 7c5075601088d1bfe49a865c31b80aa7bfaa24bf
---

# c1545-add-tool-batch-execution-mode

## Why

同一 assistant turn 可产生多个 tool call。xylitol 已有 `XyToolExecutionMode`、`XyTool::execution_mode()`、Session `tool_mode` / `set_tool_mode`，但 ReAct 将 `tool_mode` 丢弃为 `_tool_mode`，**固定顺序** `for` **+ await**。

合约 `agent-runtime` `ar3` 已允许「可并行或串行」——缺口在**可配置调度实现**与**工具分类体系**（含 MCP 硬串行），不在从零发明 MUST 许可。

本 change **刻意异于 pi**：


|      | pi                                     | xylitol（本 change）           |
| ---- | -------------------------------------- | --------------------------- |
| 默认   | `parallel`                             | `sequential`**（与今日行为一致）**   |
| 并行语义 | 预检串行后裸并发；任一 `sequential` 工具 → **整批塌缩** | **写屏障（barrier）源序窗**         |
| MCP  | 可随工具 mode 进并发                          | **一律 Barrier**；无 glob/配置名放行 |
| 目标   | 吞吐最大化                                  | 试验性加速 + 写安全；开箱零行为漂移         |




## Decisions（已全部收敛）



### D1. 产品默认 = 今日行为

- 配置 / Session 默认 `sequential`。
- `barrier_parallel` 为试验档，仅配置或 Session API 显式开启；暂无 slash/UI。



### D2. 批模式权威 = `tool_batch.mode` + Session

- `AppConfig.tool_batch.mode`：`sequential` | `barrier_parallel`（缺省 sequential）。
- Session 持同一 `BatchMode`；bootstrap 注入；`set_tool_mode`（或等价 setter）覆盖至**下轮** run（`ar6`）。
- **批模式**与**单工具并发类**拆分（见 D7）；不以散落 if 为权威。



### D3. `barrier_parallel` = 源序写屏障窗

工具仅在 MessageEnd 后执行（`ar21`）。源序切窗：连续 `ParallelSafe` 扇出 → 遇 `Barrier` 先汇聚再串行 → 继续。

```text
ojoin(read a, read b) → write x → join(read c) → bash → join(grep d)
```



### D4. 分类：内置 trait；MCP / 未知硬 Barrier


| 类              | 成员                                                     |
| -------------- | ------------------------------------------------------ |
| `ParallelSafe` | `read` / `grep` / `find` / `ls`（经 `ToolConcurrency`）   |
| `Barrier`      | `write` / `edit` / `bash`；**一切** `mcp:`*；未知名；未声明的自注册工具 |


- **MCP MUST 永远 Barrier**：调度层对 `mcp:` 前缀短路；**MUST NOT** 提供 glob / 配置名 / annotation 放行路径（本 change 与后续 Pre-1.0 均不做）。
- `FileMutationQueue` **保留**纵深防御。



### D5. 事件 / history / 钩子

- Start：实际开始时；Update/End：可完成序；history / LLM toolResult：**源序**。
- 并行窗：源序预检 → 扇出；拒绝者不进窗。



### D6. OTEL / Langfuse

并行 `tool.execute` 同属当前 `agent.iteration`；显式 parent Span；属性 `tool_batch.mode` + `tool_batch.barrier_index`；无新 XyEvent。

### D7. 类型拆分

- `BatchMode`：`Sequential` | `BarrierParallel`（Session/配置；默认 Sequential）。
- `ToolConcurrency`：`ParallelSafe` | `Barrier`（工具；MCP 固定 Barrier）。
- 迁移/别名既有 `XyToolExecutionMode`（见 design）；**禁止** Session 默认变成 Parallel。



### D8. Open Questions 关闭记录


| ID  | 结论                                                       |
| --- | -------------------------------------------------------- |
| Q1  | bash = Barrier                                           |
| Q2  | 新节 `tool_batch.mode`；Session 映射；bootstrap 注入，setter 下轮生效 |
| Q3  | **MCP 一律串行；无配置/glob/annotation 放行**（用户拍板，覆盖原「仅 glob」推荐）  |
| Q4  | 拆 `BatchMode` + `ToolConcurrency`                        |
| Q5  | 仅 span 属性，无 window XyEvent                               |
| Q6  | 无并发硬顶                                                    |
| Q7  | 保留 FileMutationQueue                                     |
| Q8  | S1–S5 MUST；S6 SHOULD                                     |




## What Changes

- 抽出可测 `tool_batch` 调度器 + ReAct 接线
- `tool_batch.mode` 配置 + Session / 组合根
- 内置并发类补齐；MCP adapter 固定 Barrier（无放行）
- OTEL 并发 tool 父子树
- BDD harness：默认同序、屏障扇出、源序 history、MCP 硬串行



## Capabilities

- `agent-runtime` / `agent-tools` / `infra-mcp` / `runtime-config` / `infra-otel`



## Impact

- 开箱无配置 = 今日串行（非 breaking）
- `barrier_parallel`：纯读窗加速；write/edit/bash/**全部 MCP** 串行汇聚



## Non-Goals

- pi 整批塌缩；流中执行；slash UI；去掉 FileMutationQueue
- **任何** MCP 并行放行机制（glob / 名名单 / annotation）
- 与 `c1540` 超时耦合
- **用户 `!` / `!!` bang**：属 host 交互 `execute_bash`，恒串行、不进 `tool_batch`（与 Agent 工具批正交）

## Open Questions

（已清空 — 见 D8。）

## Related

- `ar3` / `ar21`；pi `agent-loop.ts`（对照）；`FileMutationQueue`；`otel11`/`otel12`
- pi 体感：默认 `toolExecution=parallel`，但预检串行发 Start、模型常单 tool/turn、同 path 写队列等会使 UI 像串行（见 `design.md`）
