---
depends_on:
  - c30-align-model-registry
  - c40-align-event-bus
  - c75-align-agent-session
---

# c80-align-agent-loop: 对齐 pi AgentLoop 高级功能

## Why
当前 xylitol 的 AgentLoop 是标准 ReAct 循环：发送消息、接收响应、执行工具调用。pi 的 AgentLoop 更高级，支持：steering/followUp 消息队列、context transformation、tool execution modes（sequential/parallel）、beforeToolCall/afterToolCall hooks、getSteeringMessages/getFollowUpMessages 回调。当前缺少这些功能，无法支持高级 agent 交互场景。

## What Changes
- **增强** `src/agent/loop.rs`：
  - `AgentHooks`：before_tool_call, after_tool_call, transform_context
  - `SteeringMode` / `FollowUpMode`：all / one-at-a-time
  - `get_steering_messages()` / `get_follow_up_messages()` 回调
  - inner loop（turn 循环）和 outer loop（消息队列循环）
  - `run_agent_loop()` 和 `run_agent_loop_continue()` 两个入口
  - Tool execution modes：`sequential` / `parallel`（并行执行使用 `tokio::join!`）
  - `prepareNextTurn` 回调支持自动 retry
- 更新 BDD 测试覆盖消息队列和 hooks 场景

## Capabilities
- agent-runtime

## Impact
- 破坏性变更：`AgentLoop::run()` 签名扩展，接受 `AgentHooks` 和 `SteeringConfig`
- `run_react_loop` 内部重构为 inner/outer loop 结构
- 工具执行从顺序改为可配置的 sequential/parallel
