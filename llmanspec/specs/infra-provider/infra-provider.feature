# language: zh-CN
# capability: infra-provider
# purpose: Provider 装配与映射：协议适配实现在 xylitol-ai-bridge；主仓 AdapterXyModel 单层外壳与 DTO 映射。
# scope: src/infra/provider/
# c2833: bridge 侧适配器行为已随 package-ai-bridge 迁子根；本 cap 收窄为根侧装配/结构 mandate（one-path-one-root）。

功能: infra-provider

  @req:r1497
  规则: adapter-single-layer-wrap
    System SHALL 在 infra/provider/adapter 提供单层适配外壳 AdapterXyModel 直接包装 bridge adapter（AiBridgeLlmAdapter），经主仓 infra map 完成 DTO→XyStream 映射，并按 stream 入参分派 generate_stream / generate；MUST NOT 在 XyModel 外壳与 bridge adapter 之间保留 1:1 包装 trait 层。
    # verified-by: src/AGENTS.md
    场景: single-layer-adapter-wrap
      当 读取适配外壳结构
      那么 适配外壳仅一层且桥接 bridge adapter

  @req:r1498
  规则: openai-responses-adapter
    System SHALL 提供 OpenAiResponsesAdapter，调用 /v1/responses，对 reasoning items 发出 XyChunk::ThinkingDelta，对 message items 发出 XyChunk::TextDelta。
    # verified-by: packages/xylitol-ai-bridge/llmanspec/specs/package-ai-bridge/package-ai-bridge.feature
    场景: responses-adapter-request-body
      假如 Responses 组装且 thinking_level 为 medium 且 tools 非空
      当 构建请求体
      那么 store 为 false 且每个 tool 的 strict 为 false 且 reasoning.summary 存在且 include 含 reasoning.encrypted_content

   @req:r1502
  规则: anthropic-messages-adapter
    System SHALL 提供 AnthropicMessagesAdapter，保留现有 Anthropic SSE 解析行为，对 thinking content blocks 发出 XyChunk::ThinkingDelta。
    # verified-by: packages/xylitol-ai-bridge/src/provider/mod.rs
    场景: anthropic-messages-thinking-branch
      当 以 medium 与 off 档解析请求 thinking 并注入各族请求体
      那么 Responses 用 reasoning.effort 且 Completions 随 compat 分流且 Anthropic 随 compat 分流且 off 档省略字段

  @req:r1503
  规则: adapter-selection
    System SHALL 基于 XyModelConfig 的 kind 与 api 字段选择具体 adapter：显式可识别 api MUST 优先；省略 api 时 OpenAI 兼容 MUST 默认 openai-responses、Anthropic MUST 默认 anthropic-messages。可识别 api MUST 含 openai-responses、openai-completions、anthropic-messages；未识别字符串 MUST 静默等同省略，MUST NOT 为此单独报错或告警。AdapterKind 选择 MUST NOT 读取 WirePolicy/compat/extra_policy。
    # verified-by: packages/xylitol-ai-bridge/src/provider/mod.rs
    场景: adapter-selection-by-kind-and-api
      当 以 provider 种类读取适配器缺省
      那么 manifest 缺省按 provider 选协议族

  @req:r1504
  规则: responses-input-items-have-type
    OpenAI Responses adapter MUST 序列化每个 input item 并带显式 type 字段，匹配 Responses API schema：message items 为 {type: message, role, content: [{type: input_text|output_text, text}]}，function calls 为 {type: function_call, ...}，function outputs 为 {type: function_call_output, ...}。MUST NOT 发出无 type 的裸 {role, content} 对象：Responses API 在 input array 混入非 message items 时会拒绝 Cannot determine type of 'item'（恰发生在 tool-calling continuation round）。assistant round 仅含 tool call 无 text 时 MUST NOT 发出空 message item（仅 function_call item）。证据：convert_messages_to_input_items 将 message items 建为无 type 的 {role, content}，c375 ReAct-loop fix 后首个真实 tool continuation round 被 OpenAI 以 Cannot determine type of 'item' 拒绝；该函数无测试覆盖。

    场景: input-items-carry-type
      假如 Responses 组装且消息含 user 与 assistant
      那么 每个 input item 均带 type 字段
  @req:r1505
  规则: 单一-XyModel-装配路径
    System MUST 通过唯一装配路径将协议适配器暴露为 XyModel：具体接线实现归属 packages/xylitol-ai-bridge，经主仓映射与统一外壳（如 AdapterXyModel）注入；OpenAI 兼容 MUST 可经 Responses 或 Completions 适配暴露；MUST NOT 在主仓保留与包并行的第二套完整 adapter 实现。
    # verified-by: src/AGENTS.md
    场景: single-xy-model-assembly-path
      当 读取适配外壳结构
      那么 适配外壳仅一层且桥接 bridge adapter

  @req:r1506
  规则: vendor-类型不出-infra与agent
    async-openai 与各厂商 HTTP/SSE 具体类型 MUST 仅出现在 packages/xylitol-ai-bridge（及其测试）与必要的主仓映射边界内；agent/ MUST NOT import 这些 vendor crate；主仓 infra 若保留映射模块 MUST NOT 把 vendor 类型再导出给 agent。
    # verified-by: src/AGENTS.md
    场景: vendor-types-stay-in-bridge
      当 扫描厂商类型的出现位置
      那么 厂商类型仅现于 bridge 与映射边界

  @req:r1499
  规则: agent-projects-before-model
    XyModel 端口的消息入参 MUST 为 Vec<AiBridgeMessage>（或等价 LLM DTO），MUST NOT 再接受 AgentMessage。agent 在调用 XyModel 之前 MUST 经 project_for_llm（或等价）完成 Env 折叠；infra provider 装配 MUST NOT 再以 AgentMessage→bridge 孪生映射为主路径。投影 MUST 保留开闭：新环境消息变体的折叠策略落在 agent 投影层，MUST NOT 要求 bridge 同步增加平行 session 角色。
    # verified-by: llmanspec/specs/agent-session/agent-session.feature
    场景: model-port-takes-bridge-dto
      当 读取模型端口的消息入参形态
      那么 入参为 bridge DTO 且无 AgentMessage

  @req:r1500
  规则: wire-policy-named-compat
    主仓装配 MUST 将 YAML models.*.compat（命名轮廓，如 generic / deepseek）解析为 bridge WirePolicy 配置文件并注入 Responses/Completions；省略 compat MUST 等价 generic 默认板。MUST NOT 另建并行默认值真源；MUST NOT 接受自由形式 extra_policy YAML 或正式 env 覆盖板。由装配单测覆盖，MUST NOT 为静态缺省单独扩 BDD step。
    # verified-by: packages/xylitol-ai-bridge/src/wire_policy/mod.rs
    场景: named-compat-profile-injection
      当 以 medium 与 off 档解析请求 thinking 并注入各族请求体
      那么 Responses 用 reasoning.effort 且 Completions 随 compat 分流且 Anthropic 随 compat 分流且 off 档省略字段

  @req:r1501
  规则: api-fullname-full-names
    配置与装配中可识别的 api 字面量 MUST 使用全称 openai-responses、openai-completions 与 anthropic-messages；产品文档 MUST NOT 把配置真值简写成 responses、completions 或 messages。省略 api→默认 openai-responses（OpenAI 兼容）行为保持（见 pa4 / m13）。由单测与文档覆盖，MUST NOT 单独扩 BDD step。
# re-review(c2826): 复审结论——本 capability 管辖行为不变；分支内改动仅测试基建与可见性再导出（2026-09-28）
    场景: api-literals-full-names
      当 读取 api 字面量全称集
      那么 三全称在册且无简写别名


# re-review(c2827): 复审结论——本 capability 管辖行为不变；分支内改动为 BDD 场景落地、BDD 测试基建（steps/bindings/驱动旋钮与探针）与可见性再导出（2026-09-28）
    # verified-by: packages/xylitol-ai-bridge/src/provider/mod.rs
