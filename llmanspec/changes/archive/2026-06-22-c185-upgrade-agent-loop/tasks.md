# c185-upgrade-agent-loop: Tasks

## Dual-Loop Implementation

- [x] 重构 `run_react_loop()` 为 `run_dual_loop()` — 内层循环 + 外层循环
- [x] 内层循环：处理工具调用 + 排空 steering 消息（通过 get_steering_messages 回调）
- [x] 外层循环：内层循环完成后，检查 follow-up 队列（通过 get_follow_up_messages 回调）
- [x] 添加 `pendingMessages` 累加器（`pending_history` + outer_iterations）
- [x] 从 hooks 添加 `get_steering_messages()` / `get_follow_up_messages()` 回调
- [x] 为每个子 turn 正确发射 turn_start/turn_end

## Parallel Execution

- [x] 添加 `execute_tools_parallel()` 使用 `futures::future::join_all`
- [x] 添加 `execute_tools_sequential()` 使用基于 cancel 的 for 循环
- [x] 根据 `tool_mode` 自动选择并行/串行模式
- [x] 为每个工具发射 tool_execution_start/end
- [x] tool_execution_update 事件（已完成）

## Turn Callbacks

- [x] 添加 `NextTurnSnapshot`、`TurnContext`、`ToolCallHookResult` 类型
- [x] 添加 `PrepareNextTurn` 回调：`Fn(&TurnContext) -> Option<NextTurnSnapshot>`
- [x] 添加 `ShouldStopAfterTurn` 回调：`Fn(&TurnContext) -> bool`
- [x] 添加 `TransformContext` 回调：`Fn(Vec<AgentMessage>) -> Vec<AgentMessage>`
- [x] 通过 `ReActConfig.hooks` 从 AgentLoop 传递到循环

## Tool Hooks

- [x] 添加 `BeforeToolCall` 钩子：返回 `ToolCallHookResult::Allow | Block | Modify`
- [x] 添加 `AfterToolCall` 钩子：接收(..., result, isError) → 返回修改后的结果
- [x] 将钩子接入 `execute_single_tool`（串行和并行共享）

## Verification

- [x] `cargo build` — 0 errors
- [x] `cargo test` — 循环测试通过
- [x] `llman sdd validate c185-upgrade-agent-loop`
