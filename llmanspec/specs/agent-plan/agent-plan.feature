# language: zh-CN
# capability: agent-plan
# purpose: 计划模式：可审阅的计划文档、显式审批、模式切换与 todo 交接。
# scope: plan mode 状态机、plan 文档工件、审批与提示注入（MVP 不做工具只读限制）。

功能: agent-plan

  @req:r2948
  规则: plan-mode-state
    计划模式 须为每会话显式状态（plan / execute），以 custom_type = "plan_mode" 的 CustomEntry 只追加保存；缺省 须为 execute。用户经 `/plan` 或 `set_plan_mode` 切换 须立即写入状态，并在下一次模型调用生效；agent 经 `plan_mode` 工具请求进入 plan 须经 ask 批准，拒绝或跳过 须保持原状态。切换 须发布 `planModeChanged` 事件。模式状态 不得由用户措辞推断。

  @req:r2949
  规则: plan-document
    会话 须支持多个计划文档，以稳定 `plan_id` 区分；`plan_write` / `plan_read` / `plan_list` 须接受或返回 `plan_id`（省略时取最新/活动计划）。文档 须以 custom_type = "plan_doc" 的 CustomEntry 为 SSOT，并物化人类可读的 `plans/<plan_id>.md` 视图（会话临时目录）；视图 不得反向成为 SSOT。步骤 须含 id、content、可选 depends_on / verification；校验 须拒绝重复 id、未知依赖、自依赖与环。已批准文档被再次写入 须回到 draft，审批须重新进行。

  @req:r2950
  规则: plan-approval-handoff
    `plan_submit` 须把文档置为 awaiting_approval 并经 ask 请求用户批准；批准 须置为 approved、切换 execute 并把计划步骤交接为 todo（id/content/status 映射）；拒绝、跳过或自由文本 须回到 draft、保持 plan 模式并把用户答复返回模型。无 ask 网关时 须返回结构化错误，不得静默批准。todo 交接失败 须报告失败但不得回滚已批准的用户决定。

  @req:r2951
  规则: plan-mode-prompt
    plan 模式激活时，runtime 须在模型上下文注入计划契约：先研究与阅读，使用 `plan_write` 产出计划文档，再经 `plan_submit` 请求批准；批准前 不得声称开始编辑。本 change 只做提示注入，不限制内置工具；工具只读策略见后续 change。
