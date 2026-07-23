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

  @req:ar8
  场景: default-drain-one-at-a-time
    假如 使用缺省 QueueMode 入队两条 steer
    当 一次 drain
    那么 只取出一条且队列剩一条

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

  @req:ar25
  场景: mid-run-select-applies-next-turn
    假如 多 turn mock 且第一 turn 已开始流式
    当 run 中途 select_model 或 set_thinking_level 到新值
    那么 当前流仍用旧绑定且下一 turn 的 generate_stream 用新绑定

  @req:ar25
  场景: idle-abort-converges
    假如 run 中途切换 selected 后 abort
    当 run 结束后查询 active 与 selected
    那么 active 与 selected 收敛为同一 model 与 thinking

  @req:ar26
  场景: abort-persists-partial
    假如 模型流已输出部分正文后用户 abort
    当 检查 session history
    那么 存在 stop_reason=aborted 的 assistant 且含 partial 正文

  @req:ar26
  场景: abort-skipped-in-llm-project
    假如 history 含 stop_reason=aborted 的 assistant
    当 project_for_llm
    那么 投影结果不含该 assistant 行
