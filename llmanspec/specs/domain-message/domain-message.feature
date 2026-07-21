# language: zh-CN
# managed by llman sdd partition-migrate
功能: domain-message

  @req:dm1
  场景: roundtrip-thinking
    假如 内存中有含 Thinking 与 Text 的 AssistantMessage
    当 serde 序列化再反序列化
    那么 仍为 Thinking 与 Text 两部件且字段/type 与 pi 形一致

  @req:dm1
  场景: reject-bare-text
    假如 content 含裸字符串 Hi
    当 按 AgentPart 解析
    那么 MUST 失败或拒绝为合法 Text 部件（不静默接受）

  @req:dm1
  场景: reject-untagged-thinking
    假如 content 含 {redacted:false, text:...} 无 type
    当 按 AgentPart 解析
    那么 MUST 失败或拒绝为 Thinking（不静默接受）

  @req:dm2
  场景: prefill-skips-thinking
    假如 assistant content 含 thinking 与 text
    当 提取 message_text
    那么 结果仅含 text 正文

  @req:dm3
  场景: write-parent-id-camel
    假如 创建并 append 一条 message
    当 序列化 JSONL 行
    那么 含 parentId 键且 header version 为 5

  @req:dm4
  场景: tool-call-id-key
    假如 一条 toolResult 消息
    当 serde 序列化
    那么 JSON 含 toolCallId 且无 toolUseId

  @req:dm6
  场景: bash-is-session
    假如 一条 bashExecution 消息经 bang 落盘
    当 持久化 session
    那么 JSONL 行为 type=message 且 message.role=bashExecution（非顶层 type=bashExecution）

  @req:dm6
  场景: llm-compose
    假如 一条 user 消息
    当 序列化再反序列化
    那么 得到 AgentMessage::Llm 且叶类型为 bridge AiBridgeMessage（或等价）

  @req:dm6
  场景: compose-not-twin
    假如 审查 domain 公共 API
    当 查找平行于 AiBridgeMessage 的第二份 LLM 叶 enum
    那么 不存在（仅组合 bridge DTO + Env）

  @req:dm3
  场景: legacy-bash-lift
    假如 JSONL 含旧顶层 type=bashExecution 条目
    当 构建 AgentMessage 上下文
    那么 提升为等价 Message+role=bashExecution 且字段可读
