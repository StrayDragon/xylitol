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

  场景: 选择模型触发 model_select
    假定 注册了匹配 "model_select" 的 hook
    当 执行操作 "选择模型 fake"
    那么 hook 脚本被调用
    并且 hook 上下文包含键 "model"

  场景: 设置思考级别触发 thinking_level_select
    假定 注册了匹配 "thinking_level_select" 的 hook
    当 执行操作 "设置思考级别 high"
    那么 hook 脚本被调用
    并且 hook 上下文包含键 "level"

  场景: 打开会话树触发 session_tree
    假定 注册了匹配 "session_tree" 的 hook
    当 执行操作 "打开会话树"
    那么 hook 脚本被调用
    并且 hook 上下文包含键 "kind"

  场景: session_before_tree block 取消打开树
    假定 注册了匹配 "session_before_tree" 的 hook
    并且 hook 返回 {"action":"block","reason":"树被拒绝"}
    当 执行操作 "打开会话树"
    那么 操作失败原因包含 "树被拒绝"

  场景: 切换会话触发 session_shutdown
    假定 注册了匹配 "session_shutdown" 的 hook
    当 执行操作 "切换会话 target"
    那么 hook 脚本被调用
    并且 hook 上下文包含键 "reason"

  场景: session_before_switch block 取消切换
    假定 注册了匹配 "session_before_switch" 的 hook
    并且 hook 返回 {"action":"block","reason":"切换被拒绝"}
    当 执行操作 "切换会话 target"
    那么 操作失败原因包含 "切换被拒绝"

  场景: 执行 bash 触发 user_bash
    假定 注册了匹配 "user_bash" 的 hook
    当 执行操作 "执行 bash"
    那么 hook 脚本被调用
    并且 hook 上下文包含键 "command"

  场景: user_bash block 取消执行
    假定 注册了匹配 "user_bash" 的 hook
    并且 hook 返回 {"action":"block","reason":"bash被拒绝"}
    当 执行操作 "执行 bash"
    那么 操作失败原因包含 "bash被拒绝"

  场景: 未知库操作名可读失败
    当 执行操作 "未登记操作"
    那么 操作失败原因包含 "未知操作"
