# language: zh-CN
# capability: agent-workflow
# purpose: workflow 执行引擎：带依赖的任务图、局部重规划与审批。
# scope: DAG 校验、重规划约束、中间件状态机。

功能: agent-workflow

  @req:r2946
  规则: workflow-middleware
    workflow 中间件 须在 before_llm_request 读取最新 workflow：无 workflow 时注入一次建计划契约（要求模型调用 `workflow_write` 工具，不得输出 `<plan>` 原始块）；有 runnable 任务时注入当前任务（含 inputs 与 verification）；有失败任务时注入局部重规划指令；有 blocked 任务时注入阻塞说明。after_tool 须在 `workflow_write` 成功时从会话重新加载并采用；失败时保留旧 workflow 并在下一次 before_llm_request 注入 `<workflow_error>`；任务声明了 `tools` 时 只有该集合内工具失败才把当前 in_progress 任务标记 failed，未声明 `tools` 的旧 workflow 回退为任何工具失败即标记 failed；与任务无关的工具错误 只回写错误，不得改变任务状态。任务 completed / failed 后清除其审批。decide_continue 须满足：blocked 且未批准 → Stop；blocked 且用户有新消息 → 批准一次并 Continue；有失败且 replan 次数小于 max_replans → Continue；有 runnable → Continue；全部完成且尚未总结 → Continue 一次，已总结 → Stop；无法推进 → Stop。中间件 不得自行调用模型或工具，不得用 workflow 工具以外的入口写 workflow。max_replans 须有界并记录停止原因。容器阶段 必须 收敛为 WorkflowPhase（Contract / Executing / AwaitingApproval / Replanning / SubWorkflow / Summarizing / Done）；reload 与 after_tool 的失败标记 必须 合一；任务批准 必须 一次消费一次（解锁即清位），重跑 必须 重新批准。

    场景: workflow 中间件按状态注入
      假如 会话无 workflow 或存在可运行/失败任务
      当 运行 before_llm_request
      那么 分别注入建计划契约、当前任务或局部重规划指令

  @req:r2966
  规则: workflow-tools
    workflow 插件 须提供 `workflow_write`、`workflow_read` 与 `workflow_fail` 工具。`workflow_write` 须在工具边界校验 DAG 与重规划约束；成功 须 append `custom_type = "workflow"` 的 CustomEntry 并发布 `WorkflowUpdated`；失败 须返回 `InvalidArgs` 且不写入。`workflow_read` 须返回最新 workflow JSON。`workflow_fail` 须接受可选 `task_id`（缺省当前 `in_progress`）与必填 `reason`：目标任务非 completed 时置 failed、写 `output_ref = failure:<reason>`、append 新 workflow 条目并发布 `WorkflowUpdated`；completed、任务不存在或无 `in_progress` 时 须返回 `InvalidArgs` 且不写入。workflow 插件停用时 不得注册这三个工具或中间件。所有任务状态写入 必须 经转移表：Pending→InProgress；InProgress→Completed / Failed / Blocked；Blocked→InProgress（批准消费）；Failed→InProgress（仅 replan 入口）；Completed 锁定（replan 显式入口仅可改下游）。非法写入 必须 返回 LLM 可见错误且不写入；存量事实 必须 按表容忍投影。由插件单测覆盖。

    场景: workflow_write 后进入任务指令
      假如 workflow 插件启用且写入含一个 pending 任务的 workflow
      当 模型下一轮 before_llm_request
      那么 注入当前任务指令

    场景: workflow_fail 标记任务失败
      假如 workflow 插件启用且 workflow 含一个 in_progress 任务
      当 模型调用 workflow_fail 并给出原因
      那么 该任务状态为 failed 且下一轮 before_llm_request 注入局部重规划指令

  @req:r2969
  规则: nested-workflow
    WorkflowTask 可含 `sub_workflow`（嵌入子 workflow）。校验 须递归：每层 id 唯一、依赖存在、无环；深度与总步数 须受 `max_depth` / `max_total_steps` 约束。执行时 容器任务 须进入子 workflow（父标记 in_progress）；子全部完成时 父须标记 completed；子失败 须先在子层重规划，`max_replans` 用尽后 父须标记 failed 并向上重规划；子 blocked 须使整轮停止直至用户批准。`resume_cursor` 须为 `/` 分隔的任务路径；缺失或无效时 须回退到根的第一个未完成任务。嵌套 workflow 仍存于同一 CustomEntry，不得新增独立条目。由 SDK / 中间件单测覆盖。

    场景: 嵌套子 workflow 完成后父任务标记完成
      假如 嵌套 workflow 的子任务已完成
      当 模型下一轮 before_llm_request
      那么 注入全部完成总结
