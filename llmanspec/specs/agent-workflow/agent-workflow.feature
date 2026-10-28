# language: zh-CN
# capability: agent-workflow
# purpose: workflow 执行引擎：带依赖的步骤图、局部重规划与审批。
# scope: DAG 校验、重规划约束、中间件状态机。

功能: agent-workflow

  @req:r2946
  规则: workflow-middleware
    workflow 中间件 须在 before_model 读取最新 workflow：无 workflow 时注入一次建计划契约（要求模型输出 `<plan>{json}</plan>` 块）；有 runnable 步骤时注入当前步骤（含 inputs 与 verification）；有失败步骤时注入局部重规划指令；有 blocked 步骤时注入阻塞说明。after_model 须解析 assistant 中的 `<plan>` 块：校验 DAG 与重规划约束后 append `custom_type = "workflow"` 的 CustomEntry 并采用；非法块 须保留旧 workflow 并在下一次 before_model 注入错误说明。after_tool 须在工具失败时把当前 in_progress 步骤标记 failed；步骤 completed / failed 后清除其审批。decide_continue 须满足：blocked 且未批准 → Stop；blocked 且用户有新消息 → 批准一次并 Continue；有失败且 replan 次数小于 max_replans → Continue；有 runnable → Continue；全部完成且尚未总结 → Continue 一次，已总结 → Stop；无法推进 → Stop。中间件 不得自行调用模型或工具，不得用 workflow 工具以外的入口写 workflow。max_replans 须有界并记录停止原因。
