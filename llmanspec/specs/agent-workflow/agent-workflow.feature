# language: zh-CN
# capability: agent-workflow
# purpose: workflow 执行引擎：带依赖的步骤图、局部重规划与审批。
# scope: DAG 校验、重规划约束、中间件状态机。

功能: agent-workflow

  @req:r2946
  规则: workflow-middleware
    workflow 中间件 须在 before_llm_request 读取最新 workflow：无 workflow 时注入一次建计划契约（要求模型调用 `workflow_write` 工具，不得输出 `<plan>` 原始块）；有 runnable 步骤时注入当前步骤（含 inputs 与 verification）；有失败步骤时注入局部重规划指令；有 blocked 步骤时注入阻塞说明。after_tool 须在 `workflow_write` 成功时从会话重新加载并采用；失败时保留旧 workflow 并在下一次 before_llm_request 注入 `<workflow_error>`；步骤声明了 `tools` 时 只有该集合内工具失败才把当前 in_progress 步骤标记 failed，未声明 `tools` 的旧 workflow 回退为任何工具失败即标记 failed；与步骤无关的工具错误 只回写错误，不得改变步骤状态。步骤 completed / failed 后清除其审批。decide_continue 须满足：blocked 且未批准 → Stop；blocked 且用户有新消息 → 批准一次并 Continue；有失败且 replan 次数小于 max_replans → Continue；有 runnable → Continue；全部完成且尚未总结 → Continue 一次，已总结 → Stop；无法推进 → Stop。中间件 不得自行调用模型或工具，不得用 workflow 工具以外的入口写 workflow。max_replans 须有界并记录停止原因。

  @req:r2966
  规则: workflow-tools
    workflow 插件 须提供 `workflow_write`、`workflow_read` 与 `workflow_fail` 工具。`workflow_write` 须在工具边界校验 DAG 与重规划约束；成功 须 append `custom_type = "workflow"` 的 CustomEntry 并发布 `WorkflowUpdated`；失败 须返回 `InvalidArgs` 且不写入。`workflow_read` 须返回最新 workflow JSON。`workflow_fail` 须接受可选 `step_id`（缺省当前 `in_progress`）与必填 `reason`：目标步骤非 completed 时置 failed、写 `output_ref = failure:<reason>`、append 新 workflow 条目并发布 `WorkflowUpdated`；completed、步骤不存在或无 `in_progress` 时 须返回 `InvalidArgs` 且不写入。workflow 插件停用时 不得注册这三个工具或中间件。由插件单测覆盖。

    场景: workflow_write 后进入步骤指令
      假如 workflow 插件启用且写入含一个 pending 步骤的 workflow
      当 模型下一轮 before_llm_request
      那么 注入当前步骤指令

    场景: workflow_fail 标记步骤失败
      假如 workflow 插件启用且 workflow 含一个 in_progress 步骤
      当 模型调用 workflow_fail 并给出原因
      那么 该步骤状态为 failed 且下一轮 before_llm_request 注入局部重规划指令

  @req:r2969
  规则: nested-workflow
    WorkflowStep 可含 `sub_workflow`（嵌入子 workflow）。校验 须递归：每层 id 唯一、依赖存在、无环；深度与总步数 须受 `max_depth` / `max_total_steps` 约束。执行时 容器步骤 须进入子 workflow（父标记 in_progress）；子全部完成时 父须标记 completed；子失败 须先在子层重规划，`max_replans` 用尽后 父须标记 failed 并向上重规划；子 blocked 须使整轮停止直至用户批准。`resume_cursor` 须为 `/` 分隔的步骤路径；缺失或无效时 须回退到根的第一个未完成步骤。嵌套 workflow 仍存于同一 CustomEntry，不得新增独立条目。由 SDK / 中间件单测覆盖。

    场景: 嵌套子 workflow 完成后父步骤标记完成
      假如 嵌套 workflow 的子步骤已完成
      当 模型下一轮 before_llm_request
      那么 注入全部完成总结
