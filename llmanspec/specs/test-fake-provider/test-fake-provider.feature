# language: zh-CN
# managed by llman sdd partition-migrate
功能: test-fake-provider

  @req:r38
  场景: happy
    假如 FakeProvider 配置为仅 text 的 ScenarioStep::Text
    当 generate_content() 被调用
    那么 响应流 yield 单个含预定 text 的 LlmResponse

  @req:r38
  场景: tool-call
    假如 FakeProvider 配置 ScenarioStep::ToolCall，含 preset name 与 args
    当 generate_content() 被调用
    那么 响应含 Part::FunctionCall，name 与 args 与配置一致

  @req:r38
  场景: delay
    假如 FakeProvider 配置 ScenarioStep::Delay(50ms) 后接 text
    当 generate_content() 被调用
    那么 调用至少耗时 50ms 后返回 text 响应

  @req:r45
  场景: multi-turn
    假如 FakeProvider 配置多回合场景（text、tool_call、tool_result、text）
    当 跨对话回合调用 generate_content()
    那么 每次调用按序返回预期 step；tool_result step 被消费而不产生响应
