---
depends_on:
  - c2957-add-loop-example-plan-state
title: 进度追踪提醒：在合适时机把 todo 状态重新推给模型
---

# 进度追踪提醒：在合适时机把 todo 状态重新推给模型

## Why

今天进度追踪的链路是：

```text
todo_rewrite / todo_update 工具
  → SessionAgentTodoGateway 落会话存储
  → latest_agent_todo(entries)
  → AgentStatusBar::render() 生成 <todo> 块
  → project_outbound() 在投影时追加为 user 消息
```

也就是说，**模型只在投影时看到 `<todo>`**，中间件与 hooks 看不到结构化 todo。实际失效模式：

- 模型宣称完成，但 todo 仍有 `pending` / `in_progress`；
- 连续多轮工具调用没有更新 todo，计划变成陈旧文本；
- 工具报错后 todo 未回退，下一次决策基于错误进度；
- 人能看到 todo 栏，但模型在关键决策点没有重新注意到它。

ADPS M3 明确说：进度追踪是**给 Agent 自己用的任务台账**，不能只做成给人看的 todo list。

## What Changes（候选，promote 时逐条重估）

### 1. 结构化 todo 进入 `BeforeModelCtx`

当前 `BeforeModelCtx` 只有 `turn_index / messages / tool_schemas / options`。建议新增：

```rust
pub todo: Option<TodoList>,
```

由 core 在每次 `before_model` 前用 `latest_agent_todo(entries)` 填充。这样中间件无需解析状态栏文本，也不需要跨 crate 拿 store。

备选：把 `Arc<dyn AgentTodoGateway>` 经 `Ctx` 提供给插件，由中间件异步查询。缺点是每次 `before_model` 多一次存储读取，且中间件生命周期与 gateway 绑定更复杂。

### 2. 官方 `progress-reminder` 中间件

阶段：`before_model`（只读 + 追加 system 碎片）。

触发条件（可配置）：

| 条件 | 提醒内容 |
|---|---|
| todo 非空且最后一条 assistant 宣称完成，但仍有未完成项 | 列出未完成项，要求先更新 todo 或继续执行 |
| 连续 N 次工具调用没有 `todo_update` | 提示计划可能陈旧，要求核对/更新 |
| 上一次工具 `is_error` 且 todo 未变 | 提示失败后先回退/更新进度，再继续 |

提醒文本短、确定性、可关闭；不新增事件，不改 todo 工具 schema。

### 3. 与 Hooks 的关系

现有 `AgentHooks` 的 `after_tool_call` 能看到工具结果，`XyHookBus` 能旁路审计；但**注入模型上下文**的合适位置是 `before_model`（loop 中间件）。本 change 不把提醒塞进 hooks，避免 hooks 承担 prompt 组装职责。

## Capabilities

- `agent-runtime`：`BeforeModelCtx` 增加 todo 字段；`progress-reminder` 中间件的行为合约（何时提醒、提醒什么）。
- `agent-prompt`：提醒文本与状态栏 `<todo>` 的关系（状态栏仍是投影源，提醒是补充）。
- `agent-session-store`：无行为变更；沿用 `latest_agent_todo`。

## Impact

- crate：`xylitol-plugin-sdk`（`BeforeModelCtx` 字段）、`xylitol-runtime`（填充 todo）、`plugins/xylitol-official-plugin-progress-reminder`（新官方插件）或扩展现有 `loop-example`。
- 配置：`progress_reminder.enabled` / 阈值。
- 测试：中间件单测（触发/不触发）；BDD 用 fake todo + fake model 验证提醒进入下一轮投影。
- 文档：`src/AGENTS.md` 的 loop 边界补一句「进度提醒在 `before_model`，不是 UI 专属」。

## Out of Scope

- 自动修改 todo（仍由模型经 `todo_update` 写入）。
- 把 todo 栏做成产品 UI 的新交互。
- 用 hooks 替代中间件注入。
- 在 `before_model` 里调用模型或工具。

## Further Notes

**可复现现状证据**：

```text
plugins/xylitol-official-plugin-tools/src/tools/todo.rs
  TodoRewriteTool / TodoUpdateTool → SessionAgentTodoGateway

crates/xylitol-plugin-sdk/src/status_bar.rs
  AgentStatusBar::from_session_entries → render() → <todo>
  project_outbound() 在投影时追加为 user 消息

crates/xylitol-runtime/src/agent/runtime/middleware.rs
  BeforeModelCtx { turn_index, messages, tool_schemas, options }  ← 无 todo
```

**为什么单独一个 change**：它只解决「模型是否重新注意到进度」的问题，不改 plan-as-state 的执行语义（`c2957`）或 checkpoint（`c2960`）。三者分开，避免把提醒、执行、持久化混成一个不可验证的大 change。

**ROI**：高。提醒是纯 prompt 层，能直接降低「宣称完成但实际未完成」和「计划陈旧」两类失败；core 增量只有一个字段。
