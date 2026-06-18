# language: zh-CN
功能: 扩展系统（自定义工具 + 事件钩子）
  作为一个 LLM 代理平台
  我想要支持扩展系统
  以便第三方注册自定义工具和订阅事件钩子

  场景: 注册自定义工具
    假定 一个扩展注册了自定义工具 "greet"
    当 调用扩展工具 "greet" 参数 {"name":"Alice"}
    那么 工具返回 {"greeting":"Hello, Alice!"}

  场景: 阻塞工具调用
    假定 扩展 "block-bash" 订阅了 before_tool_call 事件
    当 尝试调用工具 "bash"
    那么 工具被阻塞
    并且 阻塞原因为 "bash is blocked by policy"

  场景: 修改工具结果
    假定 扩展 "modify-output" 订阅了 after_tool_call 事件
    当 工具 "read" 返回 {"content":"secret data"}
    那么 结果被修改为 {"content":"[redacted]"}

  场景: 扩展上下文中断信号
    假定 扩展上下文包含中断信号
    当 调用 abort
    那么 is_aborted 返回 true
