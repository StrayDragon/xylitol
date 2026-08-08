# language: zh-CN
# managed by llman sdd partition-migrate
功能: package-ai-bridge

  @req:pab1
  场景: crate-no-dep-xylitol
    假如 packages/xylitol-ai-bridge 已加入 workspace
    当 检查该包 Cargo.toml 依赖
    那么 不存在对主包 xylitol 的依赖

  @req:pab2
  场景: modules-exist
    假如 包源码树已按 design 落地
    当 枚举 src 模块
    那么 存在 provider usage accounting tokenize registry 职责边界

  @req:pab3
  场景: dto-no-bash-role
    假如 审查 bridge 公共消息 enum
    当 列出 role 变体
    那么 无 bashExecution/compactionSummary/branchSummary 平行会话角色

  @req:pab4
  场景: explicit-projection
    假如 含 BashExecution 的 AgentMessage 列表
    当 调用 project_for_llm
    那么 得到 Vec AiBridgeMessage 且无独立 bash 角色，内容按规则折叠或跳过

  @req:pab4
  场景: no-json-twin-map
    假如 生成路径装配
    当 审查 infra map 与 agent
    那么 不存在第二份平行 LLM 叶 enum，也不存在 AgentMessage↔AiBridgeMessage 全量 serde_json 往返主路径

  @req:pab5
  场景: single-impl
    假如 OpenAI 与 Anthropic 路径已迁入包
    当 审查 src/infra/provider 与包 provider
    那么 主仓无并行完整 adapter 实现体仅剩映射或装配

  @req:pab11
  场景: openai-via-sdk
    假如 Responses 请求
    当 实现归属
    那么 经 async-openai Client（非手写重复 SSE 栈为默认路径）

  @req:pab11
  场景: open-closed-adapter
    假如 新增一 OpenAI-compatible base_url
    当 接线
    那么 仅增配置/薄 adapter 不改 AgentMessage

  @req:pab12
  场景: openai-responses-effort
    假如 level=medium 且无自定义 map
    当 组装 Responses 请求
    那么 body.reasoning.effort 为 medium

  @req:pab12
  场景: anthropic-budget-off
    假如 level=off
    当 组装 Anthropic 请求
    那么 body 无 enabled thinking budget 块

  @req:pab12
  场景: map-override
    假如 map 将 high 映射为 max
    当 组装 OpenAI 类请求
    那么 effort 字段为 max

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
