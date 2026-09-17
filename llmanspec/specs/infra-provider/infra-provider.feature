# language: zh-CN
# capability: infra-provider
# purpose: Provider 装配与映射：协议适配实现在 xylitol-ai-bridge；主仓 AdapterXyModel 单层外壳与 DTO 映射。
# scope: src/infra/provider/, packages/xylitol-ai-bridge/, tests/

功能: infra-provider

  @req:r1497 @human
  场景: adapter-single-layer-wrap
    - System SHALL 在 infra/provider/adapter 提供单层适配外壳 AdapterXyModel 直接包装 bridge adapter（AiBridgeLlmAdapter），经主仓 infra map 完成 DTO→XyStream 映射，并按 stream 入参分派 generate_stream / generate；MUST NOT 在 XyModel 外壳与 bridge adapter 之间保留 1:1 包装 trait 层。

  @req:r1498 @human
  场景: openai-responses-adapter
    - System SHALL 提供 OpenAiResponsesAdapter，调用 /v1/responses，对 reasoning items 发出 XyChunk::ThinkingDelta，对 message items 发出 XyChunk::TextDelta。

  @req:r1502 @human
  场景: anthropic-messages-adapter
    - System SHALL 提供 AnthropicMessagesAdapter，保留现有 Anthropic SSE 解析行为，对 thinking content blocks 发出 XyChunk::ThinkingDelta。

  @req:r1503 @human
  场景: adapter-selection
    - System SHALL 基于 XyModelConfig 的 kind 与 api 字段选择具体 adapter：显式可识别 api MUST 优先；省略 api 时 OpenAI 兼容 MUST 默认 openai-responses、Anthropic MUST 默认 anthropic-messages。可识别 api MUST 含 openai-responses、openai-completions、anthropic-messages；未识别字符串 MUST 静默等同省略，MUST NOT 为此单独报错或告警。AdapterKind 选择 MUST NOT 读取 WirePolicy/compat/extra_policy。

  @req:r1504 @human
  场景: responses-input-items-have-type
    - OpenAI Responses adapter MUST 序列化每个 input item 并带显式 type 字段，匹配 Responses API schema：message items 为 {type: message, role, content: [{type: input_text|output_text, text}]}，function calls 为 {type: function_call, ...}，function outputs 为 {type: function_call_output, ...}。MUST NOT 发出无 type 的裸 {role, content} 对象：Responses API 在 input array 混入非 message items 时会拒绝 Cannot determine type of 'item'（恰发生在 tool-calling continuation round）。assistant round 仅含 tool call 无 text 时 MUST NOT 发出空 message item（仅 function_call item）。证据：convert_messages_to_input_items 将 message items 建为无 type 的 {role, content}，c375 ReAct-loop fix 后首个真实 tool continuation round 被 OpenAI 以 Cannot determine type of 'item' 拒绝；该函数无测试覆盖。

  @req:r1505 @human
  场景: 单一-XyModel-装配路径
    - System MUST 通过唯一装配路径将协议适配器暴露为 XyModel：具体接线实现归属 packages/xylitol-ai-bridge，经主仓映射与统一外壳（如 AdapterXyModel）注入；OpenAI 兼容 MUST 可经 Responses 或 Completions 适配暴露；MUST NOT 在主仓保留与包并行的第二套完整 adapter 实现。

  @req:r1506 @human
  场景: vendor-类型不出-infra与agent
    - async-openai 与各厂商 HTTP/SSE 具体类型 MUST 仅出现在 packages/xylitol-ai-bridge（及其测试）与必要的主仓映射边界内；agent/ MUST NOT import 这些 vendor crate；主仓 infra 若保留映射模块 MUST NOT 把 vendor 类型再导出给 agent。

  @req:r1499 @human
  场景: agent-projects-before-model
    - XyModel 端口的消息入参 MUST 为 Vec<AiBridgeMessage>（或等价 LLM DTO），MUST NOT 再接受 AgentMessage。agent 在调用 XyModel 之前 MUST 经 project_for_llm（或等价）完成 Env 折叠；infra provider 装配 MUST NOT 再以 AgentMessage→bridge 孪生映射为主路径。投影 MUST 保留开闭：新环境消息变体的折叠策略落在 agent 投影层，MUST NOT 要求 bridge 同步增加平行 session 角色。

  @req:r1500 @human
  场景: wire-policy-named-compat
    - 主仓装配 MUST 将 YAML models.*.compat（命名轮廓，如 generic / deepseek）解析为 bridge WirePolicy 配置文件并注入 Responses/Completions；省略 compat MUST 等价 generic 默认板。MUST NOT 另建并行默认值真源；MUST NOT 接受自由形式 extra_policy YAML 或正式 env 覆盖板。由装配单测覆盖，MUST NOT 为静态缺省单独扩 BDD step。

  @req:r1501 @human
  场景: api-fullname-full-names
    - 配置与装配中可识别的 api 字面量 MUST 使用全称 openai-responses、openai-completions 与 anthropic-messages；产品文档 MUST NOT 把配置真值简写成 responses、completions 或 messages。省略 api→默认 openai-responses（OpenAI 兼容）行为保持（见 pa4 / m13）。由单测与文档覆盖，MUST NOT 单独扩 BDD step。
