# language: zh-CN
# live Partitioned SSOT feature (agent-hooks)
# migrated from tests/features/hooks.feature — single G/W/T, reuse step 词表
功能: agent-hooks
  @req:r1
  场景: pre-tool-call
    假如 注册了匹配 "pre.tool_call.bash" 的 hook
    当 bash 工具即将执行
    那么 hook 脚本被调用

  @req:r3
  场景: hook-blocks
    假如 注册了匹配 pre.tool_call 的 hook 且返回 block 不允许
    当 任何工具即将执行
    那么 操作被阻止

  @req:h3
  场景: hook-modifies-args
    假如 注册了匹配 pre.tool_call.bash 的 hook 且返回 modify echo safe
    当 bash 工具以 "rm -rf /" 调用
    那么 实际执行的命令为 "echo safe"

  @req:r2
  场景: three-layer-merge
    假如 全局与用户 hook 已合并覆盖 pre.tool_call
    当 hook 被加载
    那么 使用用户配置的 hook 命令

  @req:h5
  场景: hook-timeout
    假如 hook 脚本超 2 秒且超时设为 1 秒
    当 dispatch hook
    那么 hook 在 1 秒后被杀死

  @req:h7
  场景: before-provider-request
    假如 注册了匹配 before_provider_request 的 hook 且 provider 为 deepseek
    当 provider 请求发送前
    那么 hook 收到请求 payload

  @req:h8
  场景: after-provider-response
    假如 注册了匹配 "after_provider_response" 的 hook
    当 provider 返回状态码 200
    那么 hook 收到 status=200 和响应 headers

  @req:h9
  场景: empty-hooks-noop
    假如 没有注册任何 hook
    当 任何事件触发
    那么 dispatch 是零开销 no-op
