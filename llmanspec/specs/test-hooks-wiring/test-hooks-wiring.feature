# language: zh-CN
# capability: test-hooks-wiring
# purpose: hooks 接线 BDD：观察/可取消/可修改矩阵与 XyHookBus 精选导出。
# scope: 主 crate, workspace 测试

功能: test-hooks-wiring

  @req:thw1 @human
  场景: hooks-wiring-feature
    - 仓库 MUST 提供 llmanspec/specs/test-hooks-wiring/test-hooks-wiring.feature（live Partitioned），以场景大纲固定「观察型 / 可取消」库缝接线；与 agent-hooks（调度器机制）分工：本 capability 只证 Driver/XyHookBus 权威路径，MUST NOT 用 HookDispatcher::dispatch 冒充。

  @req:thw2 @human
  场景: operation-dictionary-steps
    - BDD runner MUST 提供可复用步骤：注册匹配事件的 hook、hook 返回 JSON、执行操作「操作名」、断言 hook 被调用、断言 hook 上下文包含键、断言操作被阻止；操作名 MUST 经显式字典映射到权威路径，未知操作名 MUST 失败且信息可读。

  @req:thw3 @human
  场景: smoke-already-wired
    - 仓库 MUST 至少有一条经库缝触发的观察型例子（如 session_start 或 agent_start），经 InProcessDriver 等库缝注入 XyHookBus 并录制 hook 调用；MUST NOT 用直接 HookDispatcher::dispatch 冒充接线证明。

  @req:thw4 @human
  场景: provider-matrix-out-of-scope
    - model_select 与 thinking_level_select 已有可执行场景；OpenAI Responses / Anthropic Messages 的 provider 三缝由 agent-hooks 覆盖。本 feature MUST NOT 挂会导致 CI 失败的 provider HTTP 三缝场景。

  @req:thw5 @human
  场景: curated-xy-hook-bus
    - crate 根精选 pub use MUST 导出 XyHookBus 与 XyHookOutcome（及 NoopHookBus）；嵌入方 MUST 能在不 import protocol 深层子路径的情况下引用这些符号；MUST NOT 将 HookDispatcher 或 HookEvent 列为精选导出。

  @req:thw6 @human
  场景: wiring-model-ops
    - hooks-wiring 操作字典 MUST 支持「选择模型 fake」与「设置思考级别 high」，并启用对应观察场景。
  @executable @req:thw1
  场景: session-start
    假如 注册了匹配 "session_start" 的 hook
    当 执行操作 "确保新会话"
    那么 hook 被调用且上下文含键 reason

  @executable @req:thw6
  场景: model-select
    假如 注册了匹配 "model_select" 的 hook
    当 执行操作 "选择模型 fake"
    那么 hook 被调用且上下文含键 model

  @executable @req:thw6
  场景: thinking-select
    假如 注册了匹配 "thinking_level_select" 的 hook
    当 执行操作 "设置思考级别 high"
    那么 hook 被调用且上下文含键 level

  @executable @req:thw1
  场景: session-tree
    假如 注册了匹配 "session_tree" 的 hook
    当 执行操作 "打开会话树"
    那么 hook 被调用且上下文含键 kind

  @executable @req:thw1
  场景: tree-cancel
    假如 注册了匹配 session_before_tree 的 hook 且返回 block 树被拒绝
    当 执行操作 "打开会话树"
    那么 操作失败原因包含 "树被拒绝"

  @executable @req:thw1
  场景: session-shutdown
    假如 注册了匹配 "session_shutdown" 的 hook
    当 执行操作 "切换会话 target"
    那么 hook 被调用且上下文含键 reason

  @executable @req:thw1
  场景: switch-cancel
    假如 注册了匹配 session_before_switch 的 hook 且返回 block 切换被拒绝
    当 执行操作 "切换会话 target"
    那么 操作失败原因包含 "切换被拒绝"

  @executable @req:thw1
  场景: user-bash
    假如 注册了匹配 "user_bash" 的 hook
    当 执行操作 "执行 bash"
    那么 hook 被调用且上下文含键 command

  @executable @req:thw1
  场景: user-bash-block
    假如 注册了匹配 user_bash 的 hook 且返回 block bash被拒绝
    当 执行操作 "执行 bash"
    那么 操作失败原因包含 "bash被拒绝"

  @executable @req:thw2
  场景: unknown-op
    当 执行操作 "未登记操作"
    那么 操作失败原因包含 "未知操作"
