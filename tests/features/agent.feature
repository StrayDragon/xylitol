# language: zh-CN
功能: Agent会话
  作为一个 LLM 代理平台
  我想要完整的 Agent 生命周期管理
  以便支持 turn 事件、模型切换和思考级别控制

  背景:
    假定 有一个临时工作目录
    并且 配置了 mock 模型 "test-model"
    并且 工具注册表包含 7 个内置工具

  场景: Agent 处理纯文本响应
    假定 mock 模型返回文本 "你好，我是一个AI助手"
    当 启动 agent 会话并发送提示 "打个招呼"
    那么 响应事件流包含 TextDelta "你好"
    并且 turn_end 事件触发

  场景: Agent 处理工具调用
    假定 mock 模型返回工具调用 "read" 参数 {"path":"src/main.rs"}
    并且 read 工具返回 "hello world"
    当 启动 agent 会话并发送提示 "读取文件"
    那么 tool_execution_start 事件触发
    并且 tool_execution_end 事件包含结果 "hello world"
    并且 turn_end 事件包含 toolResult

  场景: Turn 事件顺序正确
    假定 mock 模型返回文本 "分析完成"
    当 启动 agent 会话
    那么 事件按顺序为: turn_start, message_start, message_update, message_end, turn_end

  场景: 思考级别切换
    假定 当前思考级别为 "medium"
    当 切换思考级别到 "high"
    那么 getThinkingLevel 返回 "high"
    并且 thinking_level_change 记录写入会话

  场景: 思考级别限制为模型能力
    假定 当前模型不支持思考
    当 尝试将思考级别设为 "high"
    那么 实际思考级别被限制为 "low" 或模型支持的最高级别

  场景: 运行时模型切换
    假定 注册了模型 "gpt-4o" 和 "claude-sonnet"
    并且 当前模型为 "gpt-4o"
    当 执行 cycleForward
    那么 当前模型变为 "claude-sonnet"
    并且 model_select 事件触发

  场景: 获取上下文使用量
    假定 会话包含 20000 个 token 的消息
    并且 当前模型上下文窗口为 200000
    当 调用 getContextUsage
    那么 返回 tokens 约为 20000
    并且 percent 约为 10

  场景: 会话自动持久化
    假定 一个 turn 完成
    当 加载会话文件
    那么 该 turn 的消息记录已保存
