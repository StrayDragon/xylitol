# language: zh-CN
# purpose: Provider 装配：方言插件直接 impl XyModel；组合根按 api/kind 调度工厂。
# c2833: bridge 侧适配器行为已随 provider-openai 迁子根；本 cap 收窄为根侧装配/结构 mandate（one-path-one-root）。

功能: infra-provider

  @req:r1497
  规则: adapter-single-layer-wrap
    System 须让 OpenAI Responses/Completions 与 Anthropic Messages 方言直接 impl XyModel，按 stream 入参分派流式/非流式 HTTP；不得再保留 AdapterXyModel 或 AiBridgeLlmAdapter 第三层。
    场景: 单一层适配器包裹
      当 读取适配外壳结构
      那么 适配外壳仅一层且桥接 bridge adapter

  @req:r1498
  规则: openai-responses-adapter
    System 须提供 OpenAiResponsesAdapter，调用 /v1/responses，对 reasoning items 发出 XyChunk::ThinkingDelta，对 message items 发出 XyChunk::TextDelta。
    场景: Responses适配器请求正文
      假如 Responses 组装且 thinking_level 为 medium 且 tools 非空
      当 构建请求体
      那么 store 为 false 且每个 tool 的 strict 为 false 且 reasoning.summary 存在且 include 含 reasoning.encrypted_content

   @req:r1502
  规则: anthropic-messages-adapter
    System 须提供 AnthropicMessagesAdapter，保留现有 Anthropic SSE 解析行为，对 thinking content blocks 发出 XyChunk::ThinkingDelta。
    场景: Anthropic消息思考分支
      当 以 medium 与 off 档解析请求 thinking 并注入各族请求体
      那么 Responses 用 reasoning.effort 且 Completions 随 compat 分流且 Anthropic 随 compat 分流且 off 档省略字段

  @req:r1503
  规则: adapter-selection
    System 须基于 XyModelConfig 的 kind 与 api 字段选择具体 adapter：显式可识别 api 须优先；省略 api 时 OpenAI 兼容 须默认 openai-responses、Anthropic 须默认 anthropic-messages。可识别 api 须含 openai-responses、openai-completions、anthropic-messages；未识别字符串 须静默等同省略，不得为此单独报错或告警。AdapterKind 选择 不得读取 WirePolicy/compat/extra_policy。
    场景: 适配器选区按种类与API
      当 以 provider 种类读取适配器缺省
      那么 manifest 缺省按 provider 选协议族

  @req:r1504
  规则: responses-input-items-have-type
    OpenAI Responses adapter 须序列化每个 input item 并带显式 type 字段，匹配 Responses API schema：message items 为 {type: message, role, content: [{type: input_text|output_text, text}]}，function calls 为 {type: function_call, ...}，function outputs 为 {type: function_call_output, ...}。不得发出无 type 的裸 {role, content} 对象：Responses API 在 input array 混入非 message items 时会拒绝 Cannot determine type of 'item'（恰发生在 tool-calling continuation round）。assistant round 仅含 tool call 无 text 时 不得发出空 message item（仅 function_call item）。证据：convert_messages_to_input_items 将 message items 建为无 type 的 {role, content}，c375 ReAct-loop fix 后首个真实 tool continuation round 被 OpenAI 以 Cannot determine type of 'item' 拒绝；该函数无测试覆盖。

    场景: 输入项携带类型
      假如 Responses 组装且消息含 user 与 assistant
      那么 每个 input item 均带 type 字段
  @req:r1505
  规则: 单一-XyModel-装配路径
    System 须通过唯一装配路径将协议适配器暴露为 XyModel：OpenAI 兼容接线归属 plugins/xylitol-official-plugin-provider-openai，Anthropic Messages 归属 plugins/xylitol-official-plugin-provider-anthropic；组合根按 kind/api 调用各插件工厂。不得在主仓保留与包并行的第二套完整 adapter 实现。
    场景: 单一Xy模型装配路径
      当 读取适配外壳结构
      那么 适配外壳仅一层且桥接 bridge adapter

  @req:r1506
  规则: vendor-类型不出-infra与agent
    async-openai 须仅出现在 plugins/xylitol-official-plugin-provider-openai（及其测试）；Anthropic 方言包不得依赖 async-openai。agent/ 不得 import 这些 vendor crate。
    场景: 厂商类型留在于桥缝
      当 扫描厂商类型的出现位置
      那么 厂商类型仅现于 bridge 与映射边界

  @req:r1499
  规则: agent-projects-before-model
    XyModel 端口的消息入参 须为 Vec<XyLlmMessage>（或等价 LLM DTO），不得再接受 AgentMessage。agent 在调用 XyModel 之前 须经 project_for_llm（或等价）完成 Env 折叠；infra provider 装配 不得再以 AgentMessage→bridge 孪生映射为主路径。投影 须保留开闭：新环境消息变体的折叠策略落在 agent 投影层，不得要求 bridge 同步增加平行 session 角色。
    场景: 模型端口接受桥缝DTO
      当 读取模型端口的消息入参形态
      那么 入参为 bridge DTO 且无 AgentMessage

  @req:r1500
  规则: wire-policy-named-compat
    主仓装配 须将 YAML models.*.compat（命名轮廓，如 generic / deepseek）解析为 bridge WirePolicy 配置文件并注入 Responses/Completions；省略 compat 须等价 generic 默认板。不得另建并行默认值真源；不得接受自由形式 extra_policy YAML 或正式 env 覆盖板。由装配单测覆盖，不得为静态缺省单独扩 BDD step。
    场景: 具名兼容配置档注入
      当 以 medium 与 off 档解析请求 thinking 并注入各族请求体
      那么 Responses 用 reasoning.effort 且 Completions 随 compat 分流且 Anthropic 随 compat 分流且 off 档省略字段

  @req:r2975
  规则: openai-image-projection
    OpenAI Responses/Completions 方言 须把 `AgentPart::Image` 投影进请求体，不得静默丢弃：user message 中 Responses 用 `input_image`、Completions 用 `image_url`；tool result 中 Responses 的 `function_call_output.output` 须为含 `input_text` 与 `input_image` 的数组，Completions 的 tool message 只放文本，图片须在紧邻的后续 user message 批量补发并标注来源工具调用 id，不得把图片塞进 `role=tool`。data URL 须形如 `data:<media_type>;base64,<data>`；无文本且仅图片时 须保留短占位文本。Anthropic 既有 image block 映射不变。由插件单测覆盖。

    场景: OpenAI用户图片投影
      假如 OpenAI 消息含用户图片
      当 转换为 OpenAI 请求
      那么 Responses 用户内容含 input_image 且 Completions 用户内容含 image_url

    场景: OpenAI工具结果图片投影
      假如 OpenAI 消息含工具结果图片
      当 转换为 OpenAI 请求
      那么 Responses function_call_output 含 input_image 且 Completions 以紧邻 user 消息补发图片

  @req:r1501
  规则: api-fullname-full-names
    配置与装配中可识别的 api 字面量 须使用全称 openai-responses、openai-completions 与 anthropic-messages；产品文档 不得把配置真值简写成 responses、completions 或 messages。省略 api→默认 openai-responses（OpenAI 兼容）行为保持（见 pa4 / m13）。由单测与文档覆盖，不得单独扩 BDD step。
    场景: API字面量完整名称
      当 读取 api 字面量全称集
      那么 三全称在册且无简写别名
  @req:r3010
  规则: anthropic-thinking-signature-capture
    Anthropic Messages 方言 必须 在 thinking content block 结束时发出 `XyChunk::ThinkingEnd`，携带该 block 的完整 thinking 文本与 signature；signature 必须 取自 content_block_start 的 signature 字段或 content_block_delta 的 signature_delta 累积，不得 丢弃 signature_delta。非流式响应中的 thinking block 必须 同样发出携带签名的 ThinkingEnd。由插件单测（mock SSE 与非流式 JSON）覆盖。
  @req:r3011
  规则: anthropic-thinking-replay
    Anthropic Messages 方言回放 必须 按 AgentPart::Thinking 的签名状态分派：带签名且 redacted=false 必须 映射为 {"type":"thinking","thinking","signature"}，redacted=true 必须 映射为 {"type":"redacted_thinking","data":signature}；不得 把带签名的 thinking 降级为 text block。无签名的旧 thinking 不得 伪造签名，必须 降级为 text block 并记录诊断。回放 block 顺序 必须 保持原 assistant 轮次顺序（thinking 先于 text 与 tool_use）。由插件单测覆盖。
