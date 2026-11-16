# language: zh-CN
# purpose: 请求级上下文：本轮请求可携带前序消息与 system 覆盖，作为种子历史运行，不写会话。

功能: agent-run-context

  @req:r3364
  规则: request-context-seed
    请求 必须 可携带前序 user/assistant/toolResult 消息，作为本轮种子历史；有上下文时 不得 从会话加载历史。

    场景: 上下文作为种子
      假如 请求携带两条前序消息
      当 运行一轮
      那么 模型收到的历史以这两条消息开头

  @req:r3365
  规则: seed-not-persisted
    请求上下文 不得 写入会话日志；本轮 user 与 assistant 必须 照常写入。

    场景: 上下文不落盘
      假如 请求携带一条前序 assistant 消息
      当 运行一轮并检查会话条目
      那么 会话不含该前序消息且含本轮 user 与 assistant

  @req:r3366
  规则: system-override
    请求可覆盖 system prompt；覆盖值 必须 经生成选项传给模型，不得 作为 user 消息写入历史。

    场景: 系统提示覆盖
      假如 请求携带 system prompt 覆盖
      当 运行一轮并检查生成选项
      那么 生成选项使用覆盖值且历史无 system 角色行

  @req:r3367
  规则: current-turn-persisted
    有请求上下文时，本轮 user parts 与 assistant 结果 必须 写入会话；空 assistant 可按现有语义处理。

    场景: 本轮仍落盘
      假如 请求携带上下文与一条新 user 消息
      当 运行完成
      那么 会话包含新 user 与本轮 assistant
