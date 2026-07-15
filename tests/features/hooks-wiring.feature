# language: zh-CN
功能: Hook 库缝接线
  作为 xylitol 库的嵌入方
  我想要在 Driver/agent 权威 API 上观测 hook
  以便扩展能力不依赖某一应用面（TUI/Print/Server）

  # 三类通用单元（后续 change 用场景大纲 + 例子表扩展）:
  # - 观察型：注册 → 执行库操作 → hook 被调用 + 上下文含键
  # - 可取消：block → 操作未发生（session_before_* / user_bash 等）
  # - 可修改：modify → 下游效果反映修改（provider body 等）

  场景: 确保新会话触发 session_start
    假定 注册了匹配 "session_start" 的 hook
    当 执行操作 "确保新会话"
    那么 hook 脚本被调用
    并且 hook 上下文包含键 "reason"

  场景: 未知库操作名可读失败
    当 执行操作 "未登记操作"
    那么 操作失败原因包含 "未知操作"

  # 预留例子（c996 / c998 — 未接线，勿挂 #[scenario]）:
  # | 事件 | 操作名 | 上下文键 | change |
  # | model_select | 选择模型 fake | model | c996 |
  # | thinking_level_select | 设置思考级别 high | level | c996 |
  # | before_provider_request | Completions 发送流式请求 | body | c998 |
