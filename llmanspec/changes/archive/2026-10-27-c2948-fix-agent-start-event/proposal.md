---
depends_on: []
title: ReAct 运行期补发 AgentStart 事件
branch: sdd/c2948-fix-agent-start-event
base_branch: main
base_sha: c9dcc8a47bca0d31c790b031c4023a64fbbfa3ed
---

# ReAct 运行期补发 AgentStart 事件

## Why

`XyEvent::AgentStart { session_id, model }` 在 protocol、wire、TUI bridge、Print 渲染、harness 与测试里都被消费，但 **runtime 从未发射**（`rg XyEvent::AgentStart crates/xylitol-runtime` = 0）。后果：

- TUI 只能靠 `TurnStart` 推断回合起点，`AgentStart` 分支成为死路径（bridge 单测靠手工注入才覆盖）。
- Print 的 `AgentStart` 渲染分支同样不可达。
- trace 的 `agent.turn` span 与事件流缺一个可对齐的起点。

这是既有缺陷，与 loop 中间件无关，独立成立。

## What Changes

- `run_react_loop` 在 `AgentStart` 语义点（history 装配完成、`agent.turn` span 建立后、首个 `TurnStart` 之前）发一次 `XyEvent::AgentStart`，携带 `session_id` 与当前模型 id。
- **顺序约束**：`AgentStart` 必须是本 run 事件流的第一个事件；`AgentEnd` 仍是最后一个。
- **模型口径**：模型 id 取本 run 启动时的 `current_model()`（与 `agent.turn` span 属性一致），不在流内重读。
- **兼容**：wire / protocol / bridge 无需改动（映射已存在）；只补发射点。
- 观测：`agent.turn` span 与 `AgentStart` 事件的 `session_id` 一致，供 trace 对齐。

## Out of Scope

- 改变 `AgentEnd.messages` 的面本地语义。
- 为 `AgentStart` 增加新字段（`RunId` 仍不进事件）。
- TUI 渲染改造（bridge 已能处理）。

## Capabilities

- `agent-runtime`：新增 run 事件边界 MUST + 场景（`AgentStart` 首事件、`AgentEnd` 末事件、模型 id 一致）。
- `protocol-app`：事件覆盖清单已含 `AgentStart`，无需改（仅核对）。

## Impact

- crate：`xylitol-runtime`（一处发射点）。
- 测试：BDD 既有场景断言事件序（`AgentStart` 在 `TurnStart` 前）；TUI bridge 既有注入测试不变。
- 文档：`llmanspec/specs/agent-runtime/` 增一条规则。

## Further Notes

来源：c2944 verify 的缺口 I。属独立缺陷，不依赖 c2944 的中间件语义。
