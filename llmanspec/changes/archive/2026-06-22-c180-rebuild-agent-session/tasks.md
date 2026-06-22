# c180-rebuild-agent-session: Tasks

## Subscription Model

- [x] 添加 `subscribe(handler) -> ()` 到 AgentSession（基于 EventBus.on_lifecycle）
- [x] 迁移事件系统从 `AgentEventBus`（broadcast）到 `EventBus`（channel-based）
- [x] 添加 `unsubscribe()` 方法供清理
- [x] 添加生命周期事件发射辅助方法：emit_agent_start/end, emit_model_select
- [x] 实现内部 handler：AgentEvent → 持久化 + 转发（已完成）

## Prompt Pipeline

- [x] `process_prompt()` 已支持 slash 检测 + 模板展开（c110 遗留）
- [x] 扩展命令查找实现（已完成）
- [x] 技能展开实现（已完成于 c200）
- [x] 模板展开（prompt-templates 已实现）
- [x] _runAgentPrompt 实现（已完成于 c185）

## Queue System

- [x] 添加 `steer(text, images?)` 方法（接受文字 + 可选图片附件）
- [x] 添加 `follow_up(text, images?)` 方法
- [x] 添加 `clear_queue()` → 返回并清空队列内容
- [x] 添加 `pending_message_count()` / `get_steering_messages()` / `get_follow_up_messages()`
- [x] 队列变更时自动发射 `QueueUpdate` 生命周期事件

## Auto-Retry

- [x] 实现 `_is_retryable_error(AgentMessage)`：检查 StopReason::Error + 错误文本匹配
- [x] 实现 `_will_retry_after_agent_end()`：基于 RetryState.can_retry()
- [x] 实现 `_init_retry_state()` + `_prepare_retry()`：发射 AutoRetryStart/End，backoff
- [x] _handle_post_agent_run 实现（已完成于 c185）

## Lifecycle

- [x] 实现 `dispose()`：中止 retry + bash，断开生命周期订阅，清空队列
- [x] 实现 `abort()`：取消 retry + bash，发射 AgentEnd(aborted)
- [x] _flush_pending_bash_messages 实现（已完成于 c185）
- [x] 添加 `send_custom_message()` 与 trigger_turn/deliver_as 选项

## Verification

- [x] `cargo build` — 0 errors
- [x] `cargo test` — 所有测试通过
- [x] `llman sdd validate c180-rebuild-agent-session`
