# language: zh-CN
# capability: agent-llm-projection
# purpose: "AgentMessage 会话真源与 project_for_llm 投影缝：protocol 根消息词表、agent 投影边界、对 xylitol-ai-bridge DTO 的依赖分界。"
# scope: src/protocol/, src/agent/
# c2833 拆分承接：自 packages/xylitol-ai-bridge/llmanspec/specs/package-ai-bridge 平移（bridge-rootside-rules），原文保 id。

功能: agent-llm-projection

  @req:r1543
  规则: session-vs-llm-vocab
    AgentMessage MUST 作为 session/agent 真源语义，以组合表达：Llm(AiBridgeMessage) 与 Env(EnvMessage)。物理模块 MUST 位于 protocol 根（供 ports/wire 签名与 infra 可见）；agent MUST 提供 project_for_llm 并将类型再导出。agent MAY 依赖 bridge DTO；MUST NOT 再维护平行 LLM 叶 enum；MUST NOT 依赖 bridge HTTP/vendor SDK。发往模型前 MUST 在 agent 内经 project_for_llm 得到 Vec<AiBridgeMessage>。MUST NOT 将 AgentMessage 作为 XyModel 端口入参。
    # verified-by: src/protocol/message.rs

    场景: session-vs-llm-vocab-boundary
      当 读取消息词汇分层
      那么 AgentMessage 以组合表达且协议词汇单一
  @req:r1559
  规则: 主仓映射层
    主仓 agent MUST 经 project_for_llm（或等价）将 AgentMessage 投影为 Vec<AiBridgeMessage>：Llm 臂 MUST passthrough；Env 折叠策略留在 agent。XyModel 调用方 MUST 只传递投影后的 LLM DTO。agent MAY 依赖 bridge DTO 以组合 AgentMessage::Llm；agent MUST NOT 直接依赖 xylitol_ai_bridge 的 HTTP/vendor SDK 类型。
    # verified-by: src/agent/llm_project.rs

    场景: projection-via-llm-project
      当 读取主仓投影入口
      那么 AgentMessage 经投影为协议消息
# re-review(c2835): 复审结论——本 capability 管辖行为不变；协议侧改动是 JSON-RPC 载体收口（id 逐字回显、-32600 形状、PROTOCOL_VERSION 3、删 ServerHello/ClientResponse 死变体），投影词表未动。（2026-09-29）
