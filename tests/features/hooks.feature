# language: zh-CN
功能: Hook系统
  作为一个 LLM 代理平台
  我想要事件驱动的 hook 系统
  以便拦截和修改 agent 行为

  场景: 工具调用 pre hook
    假定 注册了匹配 "pre.tool_call.bash" 的 hook
    当 bash 工具即将执行
    那么 hook 脚本被调用
    并且 hook 收到包含事件类型和参数的 JSON

  场景: Hook 阻止操作
    假定 注册了匹配 "pre.tool_call" 的 hook
    并且 hook 返回 {"action":"block","reason":"不允许"}
    当 任何工具即将执行
    那么 操作被阻止
    并且 阻止原因包含 "不允许"

  场景: Hook 修改参数
    假定 注册了匹配 "pre.tool_call.bash" 的 hook
    并且 hook 返回 {"action":"modify","args":{"command":"echo safe"}}
    当 bash 工具以 "rm -rf /" 调用
    那么 实际执行的命令为 "echo safe"

  场景: 三层 hook 合并
    假定 全局配置有 hook for "pre.tool_call"
    并且 用户配置有 hook for "pre.tool_call" 覆盖全局
    当 hook 被加载
    那么 使用用户配置的 hook 命令

  场景: Hook 超时处理
    假定 一个 hook 脚本执行超过 2 秒
    并且 hook 超时设为 1 秒
    当 dispatch hook
    那么 hook 在 1 秒后被杀死
    并且 操作被允许继续（fail-open 策略）

  场景: after_provider_request hook 用于 prefix-caching
    假定 注册了匹配 "after_provider_request" 的 hook
    并且 当前 provider 为 "deepseek"
    当 provider 请求发送前
    那么 hook 收到请求 payload
    并且 hook 可以注入 cache_control 字段

  场景: after_provider_response hook
    假定 注册了匹配 "after_provider_response" 的 hook
    当 provider 返回状态码 200
    那么 hook 收到 status=200 和响应 headers

  场景: 空 hook 配置为零开销
    假定 没有注册任何 hook
    当 任何事件触发
    那么 dispatch 是零开销 no-op
