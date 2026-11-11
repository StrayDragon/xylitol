# Design — Workflow 任务状态转移（全部已决 2026-11-11）

## 0. 上游术语 gate

本 change 的 Step → Task 更名判定依据来自两处上游交付物：c3240 design §0（三层词汇定稿：step = 一次模型调用周期；run 为宿主实现术语）与 c3240 T2（XyEvent::StepStart/StepEnd 改名落地）。检查命令：rg 'StepStart' crates/xylitol-wire/src 应命中；若无，先查 c3240 T2 是否完整实现，勿在 workflow 域自行处理撞名。

## 1. 任务转移表

~~~mermaid
stateDiagram-v2
  [*] --> Pending
  Pending --> InProgress: start
  InProgress --> Completed: finish
  InProgress --> Failed: tool_error / fail
  InProgress --> Blocked: requires_human
  Blocked --> InProgress: 用户批准（approved 消费，一次解锁一步）
  Failed --> InProgress: 重规划重启
  Completed --> [*]
  note right of Completed
    锁定：普通写入拒绝；
    下游重规划经显式 replan 入口
  end note
~~~

- 全部写入（middleware 步进 / workflow_fail 工具 / 重规划弹栈）经 task_transition(from, event) 校验；非法 → LLM 可见错误。现状核证：tools.rs:305-330 仅拦 Completed→Failed，其余无校验。
- 存量事实流容忍投影（按表投影保留最后合法态）；新写入拦截。

## 2. 容器阶段机

~~~text
WorkflowPhase::Contract        plan=None，等待 <plan> 注入
WorkflowPhase::Executing       有 plan，驱动任务
WorkflowPhase::AwaitingApproval 存在 Blocked 任务
WorkflowPhase::Replanning      失败重规划中（replan_count < max）
WorkflowPhase::SubWorkflow     stack 非空
WorkflowPhase::Summarizing     全任务完成，注入总结（取代 summary_injected/summary_done 双 bool）
WorkflowPhase::Done            总结完成
~~~

- 九字段（plan / stack / approved / replan_count / contract_injected / injected_step / summary_injected / summary_done / pending_workflow_error）降为阶段附属数据；reload 与 after_tool 失败标记合一 mark_failed；approved 随步骤重跑清位。

## 3. 更名清单（Step → Task，不留别名）

sdk session/workflow.rs（WorkflowStep → WorkflowTask、WorkflowStepStatus → WorkflowTaskStatus、字段 step_id → task_id）、workflow 中间件与 tools.rs、agent-workflow .feature（规则与场景措辞）、tests/bdd bindings、CustomEntry 持久字段名随更名（结构不变，旧条目字段名不迁移——未推广期 breaking 可接受，读取端 serde 兼容旧名的过渡解析不做）。

## 4. spec delta（landing 时）

agent-workflow 条款改写：Task 词汇 + 转移合法性（Completed 锁定 / Blocked 批准消费 / Failed 重启）+ 容器阶段语义。

## 5. 测试边界（seam）

单测：转移全矩阵（含 replan 例外入口）；容器阶段转移；重规划耗尽弹栈。BDD：agent-workflow 既有场景回归 + 非法写入拒绝（Completed 任务改写报错给 LLM）。

## 6. 风险与回滚

- 风险：阶段枚举重构触碰 middleware 主体（265-472 行区间）——先冻结行为快照测试再迁移；旧 CustomEntry 字段名不兼容（breaking 已决）。
- 回滚：转移表（sdk）/ 容器阶段（插件）/ 更名三片独立（更名宜原子合入）。

## 7. 场景走查附录（新实现者必读）

场景 A（Pending 直跳 Failed，tools.rs:305-330 已核证）：三任务计划 [分析, 修改, 验证]；分析 Completed；模型对从未 start 的「验证」（Pending）调 workflow_fail——现状唯一守卫只拦 Completed→Failed，Pending 通过，重规划被一个从未运行的任务触发。修复后经 task_transition 拒绝，错误回给 LLM。

场景 B（批准绕过）：任务2 requires_human → 用户批准 → approved.insert → 解锁；任务2 失败重跑再次 Blocked——approved 从不清空（reload 仅清 6/9 字段），静默解锁、用户被跳过。修复 = 批准一次消费一次（解锁即移除），重跑需重新批准。

场景 C（容器字段漂移）：summary_injected/summary_done 双 bool 与 reload 局部重置（6/9）是两条独立路径的漂移源——WorkflowPhase 枚举后由阶段单向转移取代，mark_failed 合一。

样例对照（测试夹具骨架）：

~~~text
Pending 直跳 Failed（前 → 后）：
  数据: steps=[s1 completed, s2 in_progress, s3 pending]
  调用: workflow_fail(step_id="s3")
  前: 仅拦 Completed → s3(未 start) 直接 Failed，重规划被无运行任务触发
  后: task_transition(Pending,Fail) 拒绝「任务未处于 in_progress」，错误回 LLM

批准绕过（前 → 后）：
  s2 requires_human → 用户批准 → approved.insert(s2) → 解锁 → 失败重跑再 Blocked
  前: approved 永存（reload 只清 6/9）→ 静默解锁，用户被跳过
  后: 解锁即 approved.remove(s2)；再次 Blocked 需重新批准

转移表全貌（验收基准）：
  Pending→InProgress；InProgress→Completed|Failed|Blocked；
  Blocked→InProgress（批准一次消费）；Failed→InProgress（仅 replan 入口）；
  Completed 锁定（replan 例外仅下游）；其余组合拒绝 + 存量容忍投影。

更名对照: WorkflowStep/WorkflowStepStatus/step_id/workflow.steps
        → WorkflowTask/WorkflowTaskStatus/task_id/workflow.tasks；
  旧 CustomEntry 字段不做兼容解析（breaking 已决）。
~~~
