# language: zh-CN
# capability: agent-hooks
# purpose: 脚本/库 hooks：工具与 provider 生命周期、XyHookBus 精选导出与可取消/可修改语义。
# scope: infra 层 hooks, agent 层 runtime hooks

功能: agent-hooks

  @req:r1 @human
  场景: Hook 分发器
    - System MUST 实现 HookDispatcher，为 10+ 事件类型执行已注册 hooks。

  @req:r2 @human
  场景: 三层配置
    - System MUST 支持 global/project/user 三层 hook 配置，后层覆盖前层。

  @req:r3 @human
  场景: 脚本执行
    - System MUST 经 stdin JSON 上下文执行 hook 脚本，并解析 stdout 的 block/allow 控制。

  @req:h1 @human
  场景: hook 事件
    - Hook 系统 MUST 支持约定事件名，含：tool_call、tool_result、context、before_provider_headers、before_provider_request、after_provider_response、agent_start、agent_end、agent_settled、turn_start、turn_end、message_start、message_end、session_start、session_shutdown，及 HookEvent 中已有 xylitol 专属事件。

  @req:h2 @human
  场景: hook 匹配
    - Hook 分发器 MUST 按 event_type.phase.qualifier 模式匹配 hooks（如 pre.tool_call.bash 匹配 bash 工具 pre 阶段调用）。

  @req:h3 @human
  场景: hook 动作
    - Hook 脚本 MUST 返回 JSON，action 为 allow/block/modify；block 停止链，modify 更新后续 hooks 的 args。

  @req:h5 @human
  场景: hook 超时
    - 每个 hook 脚本的超时 MUST 可配置为可选秒数；省略或未设置时 MUST 不设执行时限；显式正整数秒时 MUST 在到期后杀死脚本。MUST NOT 将 0 解释为无限，MUST NOT 对「未设置」强制钳制为至少 1s。

  @req:h6 @human
  场景: hook 环境
    - Hook 脚本 MUST 经 stdin 接收事件上下文 JSON，并继承配置的环境变量。

  @req:h7 @human
  场景: before_provider_request
    - before_provider_request hook MUST 在 provider JSON body 构建后、HTTP 发送前运行；Modify.args MUST 替换请求体（如 cache_control）。

  @req:h8 @human
  场景: after_provider_response
    - after_provider_response hook MUST 在 HTTP 响应头可用后、成功流/体消费前触发；上下文 MUST 含 status 与响应头；MUST NOT 要求读取 SSE body。

  @req:h9 @human
  场景: 空配置零开销
    - 未配置 hooks 时，dispatch MUST 为零开销 no-op。

  @req:h11 @human
  场景: before_provider_headers
    - before_provider_headers hook MUST 在默认 auth 头构建后、发送前运行；Modify.args.headers 对象 MUST 合并或替换请求头。

  @req:h12 @human
  场景: provider hooks 接线
    - 注入 HookDispatcher 时，OpenAI Responses、OpenAI Completions 与 Anthropic Messages adapter MUST 调用三个 provider hooks；空 dispatcher MUST 为零开销 no-op。

  @req:h13 @human
  场景: 脚本桥接 ReAct
    - Agent 附加 HookDispatcher 时，ReAct MUST 除 AgentHooks 外为 tool_call（pre）、tool_result（post）、context（pre model call）分发脚本 hooks。

  @req:h14 @human
  场景: 生命周期脚本 hooks
    - 附加 HookDispatcher 时，agent_start/agent_end/turn_start/turn_end/message_start/message_end MUST 与对应 XyEvent 发射一并分发（仅观察；fail-open）。

  @req:h16 @human
  场景: 模型与 thinking 选择接线
    - 附加 XyHookBus 时，Driver select_model 与 cycle_model MUST 在成功变更后分发 model_select；set_thinking_level MUST 分发 thinking_level_select；二者 MUST 为仅观察 fail-open；MUST NOT 依赖仅 TUI 路径。

  @req:h18 @human
  场景: 会话树切换 hooks
    - 附加 XyHookBus 时，Driver session_tree 与 travel_session_tree MUST 分发 session_before_tree（Blocked 可取消）与成功时 session_tree；switch_session MUST 分发 session_before_switch（Blocked 可取消）与成功时对 prior session 的 session_shutdown；process-quit shutdown MAY 直至库拆卸缝存在。

  @req:h19 @human
  场景: user_bash 于 execute_bash
    - 附加 XyHookBus 时，Driver execute_bash MUST 在运行命令前分发 user_bash；Blocked MUST 阻止执行并 surfaced 错误。
  @executable @req:r1
  场景: pre-tool-call
    假如 注册了匹配 "pre.tool_call.bash" 的 hook
    当 bash 工具即将执行
    那么 hook 脚本被调用

  @executable @req:r3
  场景: hook-blocks
    假如 注册了匹配 pre.tool_call 的 hook 且返回 block 不允许
    当 任何工具即将执行
    那么 操作被阻止

  @executable @req:h3
  场景: hook-modifies-args
    假如 注册了匹配 pre.tool_call.bash 的 hook 且返回 modify echo safe
    当 bash 工具以 "rm -rf /" 调用
    那么 实际执行的命令为 "echo safe"

  @executable @req:r2
  场景: three-layer-merge
    假如 全局与用户 hook 已合并覆盖 pre.tool_call
    当 hook 被加载
    那么 使用用户配置的 hook 命令

  @executable @req:h5
  场景: hook-timeout
    假如 hook 脚本超 2 秒且超时设为 1 秒
    当 dispatch hook
    那么 hook 在 1 秒后被杀死

  @executable @req:h7
  场景: before-provider-request
    假如 注册了匹配 before_provider_request 的 hook 且 provider 为 deepseek
    当 provider 请求发送前
    那么 hook 收到请求 payload

  @executable @req:h8
  场景: after-provider-response
    假如 注册了匹配 "after_provider_response" 的 hook
    当 provider 返回状态码 200
    那么 hook 收到 status=200 和响应 headers

  @executable @req:h9
  场景: empty-hooks-noop
    假如 没有注册任何 hook
    当 任何事件触发
    那么 dispatch 是零开销 no-op
