---
id: c185-upgrade-agent-loop
title: "Upgrade Agent Loop — steering/follow-up outer loop, parallel execution, full hooks"
depends_on: [c180-rebuild-agent-session]
---

## Why

当前 `loop.rs` 的 ReAct 循环过于简化：

1. **无 steering/follow-up 外层循环**：每次 agent_end 后不会检查 follow-up 队列
2. **无并行工具执行**：串行是唯一模式，`ToolExecutionMode::Parallel` 枚举存在但从未使用
3. **无 `prepareNextTurn`/`shouldStopAfterTurn`**：没有每轮回调来调整模型或决定何时停止
4. **无 `transformContext` 实现**：钩子签名定义了但从未被调用
5. **无 `beforeToolCall`/`afterToolCall` 实际调用**：钩子在 `AgentHooks` 中但循环不触发它们
6. **无 `tool_execution_update` 流式输出**：工具执行期间没有中间结果事件

pi 的 `agent-loop.ts` 实现了双层循环：内层循环处理工具调用 + steering 消息，外层循环处理后跟消息。每个工具调用通过 `tool_execution_update` 支持流式输出。

## What Changes

1. **双层循环**：内层循环（工具 + steering）→ 用户消息转发 → 外层循环（follow-up）
2. **并行工具执行**：实现并行模式，使用 `futures::future::join_all` 并发执行
3. **`prepareNextTurn` 回调**：每轮后调整模型/思维水平/上下文
4. **`shouldStopAfterTurn` 回调**：条件性提前终止
5. **`transformContext` 实现**：将消息传递给 LLM 前进行预处理
6. **`beforeToolCall`/`afterToolCall` 实现**：调用注册的钩子
7. **`tool_execution_update`**：工具通过回调流式输出部分结果
8. **工具参数验证**：执行前通过 `validate_tool_arguments()` 校验参数
9. **`prepareToolCallArguments`**：每个工具可选的参数准备（如 xml->json 转换）

## Capabilities

- agent-runtime

## Impact

- `src/agent/loop.rs`：完全重写
- `src/agent/traits.rs`：更新 `XyTool` trait 以支持 streaming callback
- `src/agent/tools/*.rs`：对支持流式的工具添加 callback 参数
- `src/agent/session.rs`：调整与新循环的交互

## Definition of Done

- [ ] 双层循环实现：内层处理工具+steering，外层处理 follow-up
- [ ] 并行工具执行可用
- [ ] `prepareNextTurn` 在每轮后调用
- [ ] `beforeToolCall`/`afterToolCall` 在工具执行前后调用
- [ ] 工具执行期间发射 tool_execution_update 事件
- [ ] `cargo test` 通过
