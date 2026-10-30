# language: zh-CN
# capability: agent-plan
# purpose: 计划模式：可审阅的计划文档、显式审批、模式切换与 todo 交接。
# scope: plan mode 状态机、plan 文档工件、审批与提示注入（MVP 不做工具只读限制）。

功能: agent-plan

  @req:r2948
  规则: plan-mode-state
    计划模式 须为每会话显式状态（plan / execute），以 custom_type = "plan_mode" 的 CustomEntry 只追加保存；缺省 须为 execute。用户经 `/plan` 或 `set_plan_mode` 切换 须立即写入状态，并在下一次模型调用生效；agent 经 `plan_mode` 工具请求进入 plan 须经 ask 批准，拒绝或跳过 须保持原状态。切换 须发布 `planModeChanged` 事件。模式状态 不得由用户措辞推断。

    场景: 计划模式状态切换
      假如 会话缺省无 plan_mode 条目
      当 用户经 /plan 切换为 plan 模式
      那么 会话追加 custom_type 为 plan_mode 的 CustomEntry 且下一次模型调用生效

  @req:r2949
  规则: plan-document
    会话 须支持多个计划文档，以稳定 `plan_id` 区分；`plan_write` / `plan_read` / `plan_list` 须接受或返回 `plan_id`（省略时取最新/活动计划）。文档 须以 custom_type = "plan_doc" 的 CustomEntry 为 SSOT，并物化人类可读的 `plans/<plan_id>.md` 视图（会话临时目录）；视图 不得反向成为 SSOT。步骤 须含 id、content、可选 depends_on / verification；校验 须拒绝重复 id、未知依赖、自依赖与环。已批准文档被再次写入 须回到 draft，审批须重新进行。

    场景: 多计划文档按 id 区分
      假如 会话已有 plan_id 为 p1 的计划文档
      当 调用 plan_write 写入 plan_id 为 p2 的文档
      那么 两个文档均以 plan_doc CustomEntry 保存且 plan_read 可按 id 读取

  @req:r2950
  规则: plan-approval-handoff
    `plan_submit` 须把文档置为 awaiting_approval 并经 ask 请求用户批准；批准 须置为 approved、切换 execute 并把计划步骤交接为 todo（id/content/status 映射）；拒绝、跳过或自由文本 须回到 draft、保持 plan 模式并把用户答复返回模型。无 ask 网关时 须返回结构化错误，不得静默批准。todo 交接失败 须报告失败但不得回滚已批准的用户决定。

    场景: 计划批准交接为 todo
      假如 计划文档处于 draft 且已调用 plan_submit
      当 用户批准该计划
      那么 文档置为 approved、模式切换 execute 且步骤映射为 todo

  @req:r2951
  规则: plan-mode-prompt
    plan 模式激活时，runtime 须在模型上下文注入计划契约：先研究与阅读，使用 `plan_write` 产出计划文档，再经 `plan_submit` 请求批准；批准前 不得声称开始编辑。本 change 只做提示注入，不限制内置工具；工具只读策略见后续 change。

    场景: plan 模式注入计划契约
      假如 会话处于 plan 模式
      当 构建本回合模型上下文
      那么 注入先研究阅读、plan_write 产出、plan_submit 请求批准的契约

  @req:r2960
  规则: plan-mode-read-only
    plan 模式激活且 read_only 开启时，before_tool 闸 须拒绝 write / edit / bash 与任何不在 allowlist 的工具；拒绝 须在工具执行前发生，返回稳定原因 `plan mode is read-only`，且 不得执行该工具。默认 allowlist 须含 read / grep / find / ls / ask / plan_* / todo_*；`plan.allowed_tools` 只 可追加。execute 模式或 read_only=false 时 闸 须失效。拒绝 不得结束本 run；模型 须在同一回合收到工具错误结果，并可改调 allowlist 内工具（如 `plan_write`）。闸 只 可拒绝，不得绕过 core 权限终裁。

    场景: 计划模式拒绝写入工具
      假如 计划模式激活且只读策略开启
      当 尝试执行 write
      那么 工具在计划模式闸被拒绝

    场景: 拒绝写入后本回合继续
      假如 计划模式激活且只读策略开启
      当 尝试执行 write
      那么 拒绝不结束本回合且可改调计划工具

    场景: 切回执行模式后写入恢复
      假如 计划模式激活且只读策略开启
      当 切回执行模式并尝试执行 write
      那么 工具在计划模式闸被允许

  @req:r2974
  规则: plan-tool-surface
    execute 模式下 模型可见工具表 须隐藏 `plan_write` 与 `plan_submit`；plan 模式下 两者 须可见。`plan_read` / `plan_list` / `plan_mode` 须始终可见。隐藏 只影响模型可见 schema，不得删除工具、不得改变权限终裁、不得绕过 before_tool 只读闸；未启用 plan 时 不得改变工具表。由插件单测覆盖。

    场景: 模式决定计划工具可见性
      假如 会话处于 execute 模式
      当 构建模型可见工具表
      那么 plan_write 与 plan_submit 隐藏而 plan_read / plan_list / plan_mode 可见

  @req:r2963
  规则: plan-document-scope
    PlanScope 须为 session / project / global；省略 scope 时 须先取活动 plan_ref 的 scope，否则 session。session 以 CustomEntry 为 SSOT；project / global 以文件（frontmatter + markdown）为 SSOT。project 须经 trust 门禁；未信任 须返回 PermissionDenied 且不得降级。文件写入 须原子（temp + rename）；失败不得留下残片。plan_list 合并 scope 并标注 scope。

    场景: 项目计划跨会话可读
      假如 已信任项目且写入 project 计划
      当 新会话按活动 plan_ref 读取
      那么 返回同一 project 计划

    场景: 未信任项目计划被拒绝
      假如 项目未信任
      当 写入 project 计划
      那么 返回 PermissionDenied 且不降级
