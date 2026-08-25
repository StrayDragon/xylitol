# language: zh-CN
# capability: package-ai-bridge
# purpose: "packages/xylitol-ai-bridge：client→LLM provider 接线与 AiBridge* DTO 边界。"
# scope: xylitol-ai-bridge 包, infra 层 provider 装配

功能: package-ai-bridge

  @req:pab1 @human
  场景: workspace-包边界
    - 仓库 MUST 提供 workspace 成员包 packages/xylitol-ai-bridge，作为 client 到上游 LLM model provider 的接线与计量库；该包 MUST NOT 依赖主 crate xylitol。

  @req:pab2 @human
  场景: 包内模块划分
    - xylitol-ai-bridge MUST 按职责划分至少包含 provider、usage、accounting、tokenize、registry 模块（名称可等价），分别承担方言接线、usage 归一化、多源计量、本地/远程分词计数与模型到分词源注册。

  @req:pab3 @human
  场景: 包内-DTO-独立
    - 包对外流式与消息类型 MUST 使用包内自有 LLM DTO（如 AiBridgeMessage / AiBridgeChunk / AiBridgeUsage）作为 LLM 叶 SSOT；该 DTO MUST 仅表达发给模型的形状，MUST NOT 再定义与主仓 Env 平行的 bashExecution / compactionSummary / branchSummary / custom 等会话环境角色；包 MUST NOT 依赖主仓 xylitol。

  @req:pab4 @human
  场景: 主仓映射层
    - 主仓 agent MUST 经 project_for_llm（或等价）将 AgentMessage 投影为 Vec<AiBridgeMessage>：Llm 臂 MUST passthrough；Env 折叠策略留在 agent。XyModel 调用方 MUST 只传递投影后的 LLM DTO。agent MAY 依赖 bridge DTO 以组合 AgentMessage::Llm；agent MUST NOT 直接依赖 xylitol_ai_bridge 的 HTTP/vendor SDK 类型。

  @req:pab5 @human
  场景: provider-实现迁入
    - OpenAI 兼容（Responses 与 Completions）与 Anthropic Messages 的流式/非流式接线实现 MUST 以 xylitol-ai-bridge 为实现归属；主仓 MUST NOT 保留与包并行的第二套完整 adapter 实现体（允许薄映射与装配）。

  @req:pab11 @human
  场景: vendor-sdk-first
    - OpenAI 兼容路径（Responses、Completions、input_tokens RemoteCount）与 Anthropic 兼容路径（Messages、count_tokens）MUST 优先经厂商官方 Rust SDK Client 接线；hooks MUST 经 SDK middleware（或文档化等价扩展点）接入且保持可移植 HeaderBag/JSON body 语义；新兼容端 MUST 以新 adapter/配置扩展（开闭），MUST NOT 为每个网关分叉改 AgentMessage 或 ReAct；省略 api 时 OpenAI 兼容默认 MUST 为 Responses，显式 openai-completions MUST 可选。

  @req:pab12 @human
  场景: thinking-params-in-request-body
    - xylitol-ai-bridge 在组装 OpenAI Responses / Completions 与 Anthropic Messages 请求体时 MUST 消费已解析的 thinking 参数：Responses 用 reasoning.effort；Completions 在 compat=generic 时用 reasoning_effort，compat=deepseek 时用 thinking.type（及可选 reasoning_effort）；Anthropic 在 compat=generic 时用 thinking budget（enabled + budget_tokens），compat=deepseek 时仅 thinking.type=enabled 且 MUST NOT 发送 budget_tokens；level 为精确 off 或映射为 null 时 MUST 省略对应字段（或显式 disabled）；用户 map 字符串 MUST 覆盖内置默认；识别内置档名（含关档 off 与 Anthropic 预算档）时 MUST 精确匹配配置/请求中的档名字面量，MUST NOT 因仅大小写或 trim 差异把 HIGH 等当成 high；MUST NOT 在无 map/默认时伪造未知厂商字段。

  @req:pab28 @human
  场景: thinking-level-resolve-exact
    - resolve_thinking_for_request（或等价）对 level 与 thinking_level_map 键 MUST 精确查找；无 map 时 OpenAI MUST 将当前 level 原串作为 effort（精确 off 则省略）；Anthropic 无 map 时仅当 level 精确等于内置名时使用内置预算，否则 MUST 可观测失败。由包内单测覆盖，MUST NOT 为大小写拒绝单独扩 BDD step。

  @req:pab29 @human
  场景: provider-http-bounded
    - provider HTTP 客户端 MUST 有连接超时与请求级等待界：流式请求以 chunk-gap idle 上界判定挂起（任何入站字节重置计时）；非流式请求有总时长上界；到期返回可分类超时错误并释放连接。

  @req:pab13 @human
  场景: assistant-stream-toolcall-lifecycle
    - Responses、Completions 与 Anthropic Messages 的流式路径 MUST 将原生工具调用映射为 AiBridgeChunk 的 ToolCallStart / ToolCallDelta / ToolCallEnd 生命周期：在首个可判别的工具相关上游事件后 MUST 发出 ToolCallStart（或同轮首次 ToolCallDelta 前已有 Start）；参数 JSON 片段 MUST 经 ToolCallDelta 增量发出且携带 parse_streaming_json 渐进 args；完整参数 MUST 在块结束时经 ToolCallEnd 发出。MUST NOT 仅在 finish_reason / output_item.done / message stop 才首次暴露工具意图。MUST NOT 在核心路径解析文本通道伪工具 XML。

  @req:pab14 @human
  场景: parse-streaming-json
    - 包 MUST 提供 parse_streaming_json（或等价）：对不完整工具参数 JSON 尽最大努力解析为 serde_json::Value，非法中间态 MUST NOT panic，可回退为空对象。

  @req:pab15 @human
  场景: responses-input-align-pi
    - 组装 OpenAI Responses 请求体时：AiBridgeGenerateOptions.system_prompt 非空 MUST 每请求前置 input 项，thinking_level 非 off 时 role 为 developer，否则为 system；assistant 历史回放 MUST 仅将 Text 部分写入 output_text；Thinking 有合法 JSON thinkingSignature 时 MUST 解析为 reasoning item 按现网顺序推入 input（同轮排在 assistant text / function_call 之前），此为唯一默认全量回放；无 signature 或非法 JSON 时 MUST 省略该 Thinking 且 MUST NOT 并入 output_text，MUST NOT 用展示用 thinking 文本冒充可回放 signature。Anthropic MUST 经 body.system 消费同一 options.system_prompt。

  @req:pab16 @human
  场景: responses-reasoning-lifecycle-align-pi
    - 组装 OpenAI Responses 请求体时 MUST 设置 store 为 false，且每个 tool 项 MUST 设置 strict 为 false；thinking_level 非 off 时 MUST 在 reasoning 对象写入 summary（默认 auto）；compat 允许时 MUST 设置 include 含 reasoning.encrypted_content，compat=deepseek（或文档化等价拒绝 include 的轮廓）时 MUST NOT 发送该 include。流式或非流式收到完整 reasoning output item 时 MUST 将其 JSON 写入 Thinking 的 thinkingSignature（经 ThinkingEnd 或文档化等价 chunk 交给调用方）；若流式 output_item.done 中缺非空 encrypted_content 而随后 response.completed 或 response.incomplete 的 response.output 同 id 项带有，MUST 在落盘材料最终确定前回填进 thinkingSignature（可再发 ThinkingEnd）；MUST NOT 仅累积 reasoning 文本而不保留可回放 signature；MUST NOT 伪造 encrypted_content。默认路径 MUST 对齐上述字段。

  @req:pab17 @human
  场景: session-vs-llm-vocab
    - AgentMessage MUST 作为 session/agent 真源语义，以组合表达：Llm(AiBridgeMessage) 与 Env(EnvMessage)。物理模块 MUST 位于 protocol 根（供 ports/wire 签名与 infra 可见）；agent MUST 提供 project_for_llm 并将类型再导出。agent MAY 依赖 bridge DTO；MUST NOT 再维护平行 LLM 叶 enum；MUST NOT 依赖 bridge HTTP/vendor SDK。发往模型前 MUST 在 agent 内经 project_for_llm 得到 Vec<AiBridgeMessage>。MUST NOT 将 AgentMessage 作为 XyModel 端口入参。

  @req:pab18 @human
  场景: responses-error-message-surface
    - OpenAI Responses 适配器在将上游错误映射为 AiBridgeError 时，若错误串含嵌入 JSON（如 content:{...}）且其中 error.message 可读，MUST 将该 message（MAY 附 type）暴露在错误文案中；MUST NOT 仅因 error.code 为整数导致 SDK 反序列化失败而只展示 deserialize 噪音、掩盖 upstream overflow 等原文。

  @req:pab19 @human
  场景: wire-policy-defaults-rs
    - 包 MUST 在 xylitol-ai-bridge 提供 WirePolicy（或等价）类型，含 compat 与仅 API req/resp 的 extra_policy；未暴露策略默认值的唯一真源 MUST 为包内 defaults.rs（或等价纯常量模块）与命名轮廓构造；WirePolicy::default() MUST 只组合该默认板。compat 默认 MUST 为 generic，并 MUST 提供 deepseek 命名轮廓；extra_policy 的 prompt_cache_usage MUST 默认 true，prompt_cache_key 与 previous_response_id MUST 默认 false。MUST NOT 经自由形式 extra_policy YAML 或业务路径散落 env::var 提供这些默认。单测 MAY 用结构体字面量覆盖。由包内单测覆盖，MUST NOT 为静态默认单独扩 BDD step。

  @req:pab20 @human
  场景: wire-extra-policy-scope
    - extra_policy MUST 仅表达 API 请求组装与响应解析/维持相关布尔（至少含 prompt_cache_usage、prompt_cache_key、previous_response_id）；MUST NOT 将 tool_search、defer_loading、状态栏等 agent/ContextPolicy 能力塞进 WirePolicy；MUST NOT 增加 reasoning_replay 或 Strip/BestEffort 类回放模式字段。适配层在对应位为 false 时 MUST NOT 假装第一语言（官方）wire 语义（例如不得假定必有 cached_tokens 或可安全发 previous_response_id）。由单测/文档场景覆盖，MUST NOT 为范围声明单独扩 BDD step。

  @req:pab21 @human
  场景: first-language-vs-dialect
    - 产品与桥接叙事 MUST 区分第一语言（厂商原生 API）与方言（他方实现该协议形状，如 DeepSeek 实现 openai-responses）。可识别 AdapterKind/api 字符串（openai-responses、openai-completions、anthropic-messages）选择协议族；MUST NOT 把第一语言称作方言。方言端默认走 compat=generic 的保守 WirePolicy，可经命名 compat 轮廓切换。由文档与单测约定覆盖，MUST NOT 单独扩 BDD step。

  @req:pab22 @human
  场景: responses-prompt-cache-read-tri-state
    - Responses usage 映射 MUST 提供可区分的 Prompt Cache 读数三态（名以实现为准，如 PromptCacheRead）：NotApplicable（expects_prompt_cache_usage 为 false）、NotReported（期望但 JSON 无 cached_tokens 细节）、Tokens(n)（含 n=0）。AiBridgeUsage.cache_read（或等价）MAY 保留为派生 u64（仅 Tokens(n)→n，其余→0）供 accounting。MUST NOT 将「字段缺失」映射为「命中 0」。由包内单测覆盖，MUST NOT 单独扩 BDD step。

  @req:pab23 @human
  场景: prompt-cache-usage-obs-honesty
    - 当 ProviderRequestTrace（或等价）激活并附着 usage 时，MUST 将 Prompt Cache 三态透出到观测属性（Langfuse/trace 自定义 property 或等价）；langfuse.observation.usage_details 中的 cache_read 数字 MUST 仅在 Tokens(n) 语义下写入。MUST NOT 在 NotReported 或 NotApplicable 时写入 cache_read:0 冒充已回报命中。由包内单测覆盖，MUST NOT 单独扩 BDD step。本 req 不要求 TUI chrome。

  @req:pab24 @human
  场景: responses-assembler-sole-body
    - 包 MUST 提供 ResponsesAssembler（或等价）作为构造 openai-responses 请求 JSON body 的唯一业务布局入口：消费投影后的 AiBridgeMessage、工具 schema、AiBridgeGenerateOptions 与 WirePolicy；默认档 MUST 等价于既有 assemble_responses_body / prepend_system_prompt_item / apply_responses_wire_policy 行为（允许规范化后比较）。Adapter 内 MUST NOT 二次重排业务布局（WirePolicy 字段子集差异除外）。单测 MAY 用不同 WirePolicy 字面量覆盖证明字段集 diff。由包内 golden/单测覆盖，MUST NOT 单独扩 BDD step。

  @req:pab25 @human
  场景: responses-reasoning-full-replay-only
    - 从 session 形 AiBridgeMessage（含 thinkingSignature）经 ResponsesAssembler 重建 input 时，回放策略 MUST 仅为全量回放：有合法 signature 则原样推入 reasoning item；MUST NOT 提供 Strip/BestEffort 产品旋钮或 ExtraPolicy 回放模式枚举；空字符串 encrypted_content 的 signature MUST 仍整包回放。由包内 golden/单测覆盖，MUST NOT 单独扩 BDD step。

  @req:pab26 @human
  场景: responses-reasoning-encrypted-backfill
    - 流式 Responses 路径在 response.completed 或 response.incomplete 时，若能按 reasoning id 将终端 output 中的非空 encrypted_content 合并进先前 ThinkingEnd 的 thinkingSignature，MUST 在随后的 Done 之前发出更新后的 ThinkingEnd（或文档化等价）；主路径 MUST NOT 因缺 encrypted 而崩溃。由包内合成 SSE 单测覆盖，MUST NOT 单独扩 BDD step。

  @req:pab27 @human
  场景: responses-assemble-prefix-idempotent
    - 对同一 Vec<AiBridgeMessage>、同一 AiBridgeGenerateOptions（含 system_prompt）、同一 tools schema 与 WirePolicy，ResponsesAssembler 构造的 openai-responses 请求体中业务布局（至少 input 项序列与 tools）MUST 规范化后幂等：连续组装两次相等；消息经 serde 或 JSONL 行往返后再组装 MUST 仍得相等前缀。MUST NOT 因仅序列化往返而重排或改写历史 input。由包内单测与维护 lab（lab_session_prefix_idempotency）覆盖，MUST NOT 单独扩 BDD step。

  @executable @req:pab13
  场景: responses-toolcall-streams-before-done
    假如 Responses SSE 含 function_call 的 output_item.added 与多帧 function_call_arguments.delta 后才有 output_item.done
    当 映射为 AiBridgeChunk 流
    那么 首个 args delta 之前或当时已有 ToolCallStart 且存在至少一次 ToolCallDelta 早于对应 ToolCallEnd

  @executable @req:pab14
  场景: partial-args-object
    假如 输入残缺工具参数 JSON
    当 调用 parse_streaming_json
    那么 返回 Value 且不 panic

  @executable @req:pab15
  场景: responses-system-as-developer
    假如 Responses 组装且 system_prompt 非空且 thinking_level 为 medium
    当 转换为 input items
    那么 首项 role 为 developer 且 content 为 system_prompt

  @executable @req:pab15
  场景: responses-thinking-not-in-output-text
    假如 assistant 含 Thinking 无 signature 与 Text
    当 转换为 Responses input
    那么 output_text 仅含 Text 且无 Thinking 正文

  @executable @req:pab16
  场景: responses-body-store-strict-summary-include
    假如 Responses 组装且 thinking_level 为 medium 且 tools 非空
    当 构建请求体
    那么 store 为 false 且每个 tool 的 strict 为 false 且 reasoning.summary 存在且 include 含 reasoning.encrypted_content

  @executable @req:pab16
  场景: responses-reasoning-item-sets-thinking-signature
    假如 Responses 流或非流输出含完整 type=reasoning 的 output item
    当 映射为 AiBridgeChunk
    那么 存在带 thinkingSignature 的 Thinking 终态（ThinkingEnd 或等价）且 signature 可 JSON 解析为该 reasoning item
