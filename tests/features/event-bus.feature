# language: zh-CN
功能: EventBus 事件总线
  作为一个可扩展的 agent 系统
  我想要 channel-based 事件总线
  以便支持按事件类型订阅和扩展系统集成

  背景:
    假定 有一个新的 EventBus 实例

  场景: 按 channel 发布和订阅事件
    当 在 channel "tool_execution_start" 发布数据 "tool:bash"
    那么 subscriber 应该收到 channel "tool_execution_start" 的数据 "tool:bash"

  场景: 取消订阅后不再收到事件
    假定 subscriber 订阅了 channel "turn_end"
    当 subscriber 取消订阅
    并且 在 channel "turn_end" 发布数据 "done"
    那么 subscriber 不应该收到任何数据

  场景: clear 清除所有监听器
    假定 three subscribers across three channels
    当 调用 clear
    那么 no handlers remain

  场景: handler 错误不影响其他 handler
    假定 handler A 会 panic 而 handler B 正常 on same channel
    当 发布事件
    那么 handler B 仍然收到事件
