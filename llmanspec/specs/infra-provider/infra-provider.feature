# language: zh-CN
# managed by llman sdd partition-migrate
功能: infra-provider

  @req:pa1
  场景: trait-exists
    假如 引入新 adapter 层
    当 LlmAdapter trait 被定义
    那么 agent 循环可在任意 adapter 上调用 generate_stream 而无需命名具体类型

  @req:pa2
  场景: responses-reasoning
    假如 reasoning-capable model 经 /v1/responses 服务
    当 OpenAiResponsesAdapter 流式响应
    那么 XyChunk::ThinkingDelta items 在 XyChunk::TextDelta items 之前发出

  @req:pa3
  场景: anthropic-thinking
    假如 model 返回 thinking content blocks
    当 AnthropicMessagesAdapter 流式响应
    那么 thinking blocks 发出 XyChunk::ThinkingDelta items

  @req:pa4
  场景: default-selection
    假如 model config 省略 api 字段
    当 config loader 解析默认值
    那么 openai kind 默认 openai-responses，anthropic kind 默认 anthropic-messages

  @req:pa5
  场景: every-item-declares-type
    假如 对话含 user、assistant、tool_call、tool_result 消息
    当 转换为 Responses input items 并检查 items
    那么 每个 item 有 type 字段（message 或 function_call 或 function_call_output）且 message content 为 typed text parts 数组

  @req:pa5
  场景: tool-continuation-not-rejected
    假如 tool-calling continuation round 请求发往 Responses API
    当 请求体为 converted items
    那么 被接受（无 Cannot determine type of 'item' 错误），因每个 item 声明 type

  @req:pa6
  场景: single-path-via-bridge
    假如 变更完成后
    当 审查从配置到 dyn XyModel 的装配
    那么 仅一条路径且实现体来自 xylitol-ai-bridge 经映射注入

  @req:pa6
  场景: no-dual-impl
    假如 OpenAI 兼容与 Anthropic 装配可用
    当 搜索主仓与包内的重复 adapter 实现
    那么 主仓无并行完整实现体且无 Chat Completions 适配实现

  @req:pa20
  场景: bash-folded
    假如 history 含 bashExecution
    当 agent 投影后调用 XyModel
    那么 入参为 AiBridgeMessage 列表且无 bashExecution 变体

  @req:pa20
  场景: model-port-llm-dto-only
    假如 审查 XyModel::generate_stream 签名
    当 查看消息参数类型
    那么 为 Vec AiBridgeMessage（或等价 LLM DTO）而非 AgentMessage
