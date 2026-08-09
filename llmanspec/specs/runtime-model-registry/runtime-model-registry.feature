# language: zh-CN
# managed by llman sdd partition-migrate
功能: runtime-model-registry

  @req:m1
  场景: register-provider
    假如 以名称 openai 与 api key 注册 provider
    当 对 openai 模型调用 has_configured_auth
    那么 返回 true

  @req:m1
  场景: no-auth
    假如 anthropic 未配置 api key
    当 对 claude 模型调用 has_configured_auth
    那么 返回 false

  @req:m1
  场景: resolved-auth
    假如 api key 为已设置的环境变量引用 $MY_KEY
    当 对 provider 调用 has_resolved_auth
    那么 返回 true

  @req:m1
  场景: unresolved-auth
    假如 api key 为未设置的环境变量引用 $MY_KEY
    当 对 provider 调用 has_resolved_auth
    那么 返回 false

  @req:m2
  场景: available-models
    假如 两个 provider 各有 2 个模型
    当 调用 get_available
    那么 按优先级返回 4 个模型

  @req:m3
  场景: real-values
    当 对支持的 provider 调用 default_model_id_for_provider
    那么 返回真实可用模型标识而非 gpt-5.4 等占位符

  @req:m12
  场景: no-silent-env-select
    假如 仅设置 OPENAI_API_KEY 且无配置模型且未传 --model
    当 bootstrap 尝试装配
    那么 MUST 硬失败且 ModelRegistry MUST NOT 含 gpt-4o 或其它仅由 env 注入的默认模型
    并且 当前选中模型 MUST NOT 被自动设为 gpt-4o

  @req:m4
  场景: exact-match
    假如 可用模型含 openai/gpt-4o
    当 调用 resolve_model('openai/gpt-4o')
    那么 返回匹配模型

  @req:m5
  场景: scoped-cycle
    假如 scoped_models 有 3 项
    当 调用 cycle_model
    那么 仅在这 3 个模型间循环

  @req:m6
  场景: fallback-model
    假如 用户在 openai provider 请求 nonexistent-model
    当 解析模型
    那么 回退使用 openai 基础模型且以请求 id 为名称

  @req:m7
  场景: auth-guidance-message
    假如 anthropic 模型未配置 API key
    当 调用 auth_guidance_message
    那么 消息含 ANTHROPIC_API_KEY

  @req:m9
  场景: default-off-when-unset
    假如 模型 thinking true 且无 thinking_levels
    当 resolve_model_meta
    那么 thinking_levels 仅为 off

  @req:m9
  场景: explicit-vendor-list
    假如 配置 thinking_levels 为 off 与 high 与 max
    当 resolve_model_meta
    那么 列表为 off 与 high 与 max 且顺序一致

  @req:m9
  场景: freeform-level-name
    假如 配置 thinking_levels 含厂商字面量 xhigh
    当 resolve_model_meta
    那么 支持集含 xhigh

  @req:m10
  场景: reject-unsupported
    假如 当前模型支持集为 off 与 high
    当 set_thinking_level 为 max
    那么 失败且当前 level 不变

  @req:m15
  场景: reject-case-variant
    假如 当前模型支持集为 off 与 high
    当 set_thinking_level 为 HIGH
    那么 失败且当前 level 不变

  @req:m10
  场景: default-last-on-select
    假如 目标模型支持 off 与 high 与 max
    当 select_model 到该模型
    那么 thinking level 为 max 而非 Settings 低档默认

  @req:m10
  场景: no-thinking-is-off
    假如 目标模型 thinking 不可调或支持集仅 off
    当 select_model 到该模型
    那么 thinking level 为 off

  @req:m10
  场景: resume-restores-exact-level
    假如 会话分支末次 thinkingLevelChange 为 high 且当前模型支持集含 high
    当 resume 或装载会话上下文
    那么 当前 thinking level 为 high 且未因装载追加新的 thinkingLevelChange

  @req:m10
  场景: resume-sticky-out-of-set
    假如 会话末次 thinkingLevelChange 为 high 且当前模型支持集仅为 off 与 max
    当 resume 装载
    那么 内存 thinking level 仍为 high 且会话文件未被改写为 max 或 off

  @req:m11
  场景: level-reaches-options
    假如 当前 thinking level 为 high
    当 发起一轮 generate_stream
    那么 调用携带 thinking_level 字符串 high（或等价 options）

  @req:m11
  场景: map-from-meta
    假如 模型 meta 含 thinking_level_map
    当 发起 generate_stream
    那么 options 含该 map
