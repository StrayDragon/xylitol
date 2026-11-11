# Tasks — ReAct 内环状态（词汇定稿 + 改名 + 缺陷修复）

> 实施者导读：先读 design §0 三层词汇（run 为宿主实现术语）与 §7 三项 breaking。改名片（T2）宜原子合入。收口走 finalize，不列为任务。

- [ ] T1 spec 先行改写（design §6 specs 清单）
  agent-runtime：r1027 改写为 max_steps_per_turn 经 should_stop_after_step 生效（未配置不启用）、原 ar185 settle 钩子条款重写为 step 钩子（含 ContinueTools 调用、Settle 仍做结算）、r1036 Turn* 改 Step*、新增 AutoRetry 语义条款与 AgentEnd.reason 条款；agent-hooks r1000 事件名改 step_start / step_end；runtime-config / workspace-assembly / cli-print / protocol-app / app-tui-bridge 同步。验证：llman-sdd validate --specs --strict。
- [ ] T2 词汇改名清扫（design §6，一步到位不留别名）[blocked-by: T1]
  XyEvent::TurnStart/TurnEnd → StepStart/StepEnd、turn_index → step_index、LoopStage::BeforeTurn/AfterTurn → BeforeStep/AfterStep（含 BeforeTurnCtx / AfterTurnCtx 类型）、obs TurnEndReason → StepEndReason、turn_end.rs → step_end.rs、script_hook_ctx 与 hooks 插件事件名字符串（turn_* → step_*；agent_* → run_*）、全部消费端（wire / protocol / TUI / print / 插件 middleware）与 BDD 步骤词。附带缝字段迁移：BeforeStepCtx（原 BeforeLlmRequestCtx）增 has_new_user_messages: bool（语义与 DecideContinueCtx 同名同义——本 step 是否消费新用户输入；下游 c3248 / c3260 的 turn 边界判定依赖此字段）。验证：rg 清单归零（排除 archive 与本 change）；just fmt / just lint。
- [ ] T3 max_steps_per_turn 机制 [blocked-by: T1, T2]
  移除 session.max_turns / should_stop_after_turn / max_turns_stop_hook / settles 计数；新单槽 should_stop_after_step（每次 step 收尾调用，含 ContinueTools；ctx 含 step_index / is_settle）；config 键替换，旧键报错指路；kernel 装配接线。单测：上限触发即停且不 abort 已完成消息、未配置不启用。
- [ ] T4 RetryState 修复 [blocked-by: T2]
  每次模型调用边界重建；两个 AutoRetryEnd 发射点加真实重试守卫。单测：第 1 step 重试 + 第 2 step 顺利，断言无 AutoRetryEnd。
- [ ] T5 bang 忙拒 + nested RAII [blocked-by: T2]
  bang cancel 槽占用时 execute 返回 busy；nested 计数改 guard（Drop 递减）。单测：并发忙拒、guard 早退路径不泄漏额度。
- [ ] T6 AbortReason 底座 + AgentEnd.reason [blocked-by: T2]
  内部 AbortReason 枚举；turn_aborted 十六处写点改 abort(reason)（首个非 OK 胜出）；XyEvent::AgentEnd 增 reason 可选字段；wire 双向映射。单测：归因矩阵。
- [ ] T7 端透出 [blocked-by: T6]
  TUI bridge handler + 通知文案（走 app-tui-fixed-zone 固定词）；print 一行；BDD：UserCancel / ModelError / StepLimit 三归因端可见场景。
- [ ] T8 术语与文档 [blocked-by: T2]
  根 / src AGENTS.md 增三层词汇（session / turn 问答轮 / step）定义 + run 宿主实现术语注记（含反向提示：run 当概念层 = 旧稿）；release note 草稿（三项 breaking）。验证：AGENTS 评审过目。
- [ ] T9 全量回归 [blocked-by: T1, T2, T3, T4, T5, T6, T7, T8]
  just fmt / just lint / just test；llman-sdd validate --all --strict；BDD check_command 全绿；改名残留 rg 终检。
