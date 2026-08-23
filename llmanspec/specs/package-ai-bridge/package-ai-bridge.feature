# language: zh-CN
# managed by llman sdd partition-migrate
功能: package-ai-bridge

  @req:pab13
  场景: responses-toolcall-streams-before-done
    假如 Responses SSE 含 function_call 的 output_item.added 与多帧 function_call_arguments.delta 后才有 output_item.done
    当 映射为 AiBridgeChunk 流
    那么 首个 args delta 之前或当时已有 ToolCallStart 且存在至少一次 ToolCallDelta 早于对应 ToolCallEnd

  @req:pab14
  场景: partial-args-object
    假如 输入残缺工具参数 JSON
    当 调用 parse_streaming_json
    那么 返回 Value 且不 panic

  @req:pab15
  场景: responses-system-as-developer
    假如 Responses 组装且 system_prompt 非空且 thinking_level 为 medium
    当 转换为 input items
    那么 首项 role 为 developer 且 content 为 system_prompt

  @req:pab15
  场景: responses-thinking-not-in-output-text
    假如 assistant 含 Thinking 无 signature 与 Text
    当 转换为 Responses input
    那么 output_text 仅含 Text 且无 Thinking 正文

  @req:pab16
  场景: responses-body-store-strict-summary-include
    假如 Responses 组装且 thinking_level 为 medium 且 tools 非空
    当 构建请求体
    那么 store 为 false 且每个 tool 的 strict 为 false 且 reasoning.summary 存在且 include 含 reasoning.encrypted_content

  @req:pab16
  场景: responses-reasoning-item-sets-thinking-signature
    假如 Responses 流或非流输出含完整 type=reasoning 的 output item
    当 映射为 AiBridgeChunk
    那么 存在带 thinkingSignature 的 Thinking 终态（ThinkingEnd 或等价）且 signature 可 JSON 解析为该 reasoning item
