# language: zh-CN
# live Partitioned SSOT feature (agent-runtime)
# executable GWT only; architectural/static scenarios stay in spec.toon as feature:false
功能: agent-runtime
  @req:ar1
  @req:ar-pilot
  场景: react-terminates
    假如 mock 模型先 tool 后无 tool
    当 运行 AgentRuntime
    那么 先执行工具再结束且无 adk 类型

  @req:ar2
  @req:ar-pilot
  场景: stream-is-xyevent
    假如 消费事件流
    当 轮询
    那么 每项为 XyEvent

  @req:ar3
  场景: continues-after-tools
    假如 mock 模型先 tool 后无 tool
    当 运行 AgentRuntime
    那么 turn_end 事件包含 toolResult

  @req:ar21
  场景: intent-before-execution
    假如 mock 模型先 tool 后无 tool
    当 运行 AgentRuntime 并收集事件
    那么 MessageUpdate 含工具意图且早于任意 ToolExecutionStart；ToolExecutionStart 不早于 MessageEnd

  @req:ar12
  场景: abort-drops-sse
    假如 配置了 mock 模型 test-model 且慢速流式 40 段间隔 20 毫秒
    当 经 Driver 启动会话并在首个 TextDelta 后 abort
    那么 事件流包含 aborted 错误

  @req:ar8
  场景: steer-before-model
    假如 装配并运行入队 steer 的 agent
    当 检查队列与历史
    那么 steer 计数归零且已处理

  @req:ar8
  场景: followup-extends
    假如 装配无工具 agent 并入队 follow_up
    当 运行至将结束
    那么 继续循环而非 AgentEnd

  @req:ar9
  场景: queue-update
    假如 装配 agent 并入队 steer
    当 观察事件流
    那么 出现 QueueUpdate 且计数正确

  @req:ar10
  场景: abort-clears-steer
    假如 装配 agent 并入队 steer 与 follow_up
    当 abort
    那么 steer 空且 follow_up 保留

  @req:ar11
  场景: second-run-after-abort
    假如 装配慢速 agent 并在首轮 abort 后
    当 再次运行
    那么 正常完成而非立即 aborted

  @req:ar10
  场景: abort-cancels-bang
    假如 启动交互 bang 长命令后 abort
    当 检查 bash 结果
    那么 cancelled 为 true

  @req:ar7
  场景: before-denies
    假如 注册匹配 bash 的 before 拒绝 hook
    当 运行 AgentRuntime 触发 bash
    那么 tool-error 回写且未执行

  @req:ar23
  场景: persist-done-usage
    假如 mock 模型流以 Done 结束且携带非空 usage
    当 运行 AgentRuntime 并检查会话持久化的 assistant 消息
    那么 usage 字段非空且与 Done 一致

  @req:ar24
  场景: should-stop-emits-agent-end
    假如 注册 should_stop_after_turn 在首次 TurnEnd 后返回 true
    当 运行 AgentRuntime
    那么 出现 AgentEnd 且其后无新的模型轮 TurnStart

  @req:ar8
  @req:ar24
  场景: should-stop-skips-followup
    假如 入队 follow_up 且 should_stop_after_turn 在首次 TurnEnd 后返回 true
    当 运行 AgentRuntime
    那么 本 run 以 AgentEnd 结束且 follow_up 未被注入历史

  @req:ar24
  场景: no-hook-open-end
    假如 未注册 should_stop_after_turn 的无工具 agent
    当 运行 AgentRuntime
    那么 正常出现 AgentEnd 且恰好一轮 TurnStart
