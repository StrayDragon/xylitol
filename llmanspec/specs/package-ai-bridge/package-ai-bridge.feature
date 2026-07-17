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
    那么 得到不含独立 bash 角色的 LLM DTO 且内容按规则折叠或跳过

  @req:pab4
  场景: no-json-twin-map
    假如 生成路径装配
    当 审查 infra map
    那么 不存在 AgentMessage 与全量 AiBridgeMessage 的 serde_json 往返

  @req:pab5
  场景: single-impl
    假如 OpenAI 与 Anthropic 路径已迁入包
    当 审查 src/infra/provider 与包 provider
    那么 主仓无并行完整 adapter 实现体仅剩映射或装配

  @req:pab11
  场景: openai-via-sdk
    假如 Completions 或 Responses 请求
    当 实现归属
    那么 经 async-openai Client（非手写重复 SSE 栈为默认路径）

  @req:pab11
  场景: open-closed-adapter
    假如 新增一 OpenAI-compatible base_url
    当 接线
    那么 仅增配置/薄 adapter 不改 AgentMessage

  @req:pab12
  场景: openai-completions-effort
    假如 level=medium 且无自定义 map
    当 组装 Completions 请求
    那么 body 含 reasoning_effort 为 medium

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
