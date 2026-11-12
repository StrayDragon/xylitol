# Design — P0 批次复核收尾（全部已决除 W7）

## 0. 基线

上游 9 change 已归档合入；词汇与 spec 以当前仓库为基线（三层词汇已落地）。各项「前」均为已核证的现状（file:line 见三组 review 报告，存 research/review-findings.md）。

## 1. C1 workflow_write 绕表（CRITICAL）

- 前：write 工具 schema 直收逐任务 status（tools.rs:149）；validate_workflow_replan 只比结构不比状态（workflow.rs:442-452）→ Pending→Completed 等可落盘，违反 r2966。
- 后：write 路径逐任务按 task_transition 校验状态增量；replan 允许集例外（Completed 下游经 replan 可改）。
- 优劣：经表校验保留合法 replan 写入（推荐）；备选「write 禁收 status」更严但破坏 replan。

## 2. W1 RunCoordinator 收口

- 前：相位修改分散；mark_streaming 在 Cancelling(run_id) 仍覆写为 Streaming（state.rs:150-154，已核证竞态）。
- 后：transition(RunEvent) 单入口（Begin/StreamStarted/Cancel/Finish）+ Cancelling 守卫；c3240 design 承诺、tasks 漏列。
- 优劣：单入口防再散（推荐）；备选仅加守卫（最小，但债仍在）。

## 3. W2 存量容忍投影

- 前：project_goals / project_schedules 对状态变更事实直写 builder.state（sdk automation.rs:476-479），无过滤无 warn。
- 后：投影按 goal_transition / schedule_transition 过滤非法事实，保留最后合法态 + warn；新写入已由表拦，本项补读路径。
- 优劣：对齐 agent-goal.feature:24；备选改 spec 删条款（旧会话非法序列真实存在，不取）。

## 4. W3 TodoStatus 未知值

- 前：from_value::<TodoStatus>().ok() 后未知值落入 pending 计数（todo middleware.rs:92-98）。
- 后：解析失败显式报错回模型（InvalidArgs 风格，模型可自纠）。
- 优劣：对齐 agent-todo.feature:139；备选 warn+pending 违反条款。

## 5. W5 manifest 延迟落盘

- 前：每次 append 都 create+sync_all manifest（append.rs:325,534-550）——每 entry 双 fsync，违背「manifest 是可丢弃缓存、延迟落盘」设计。
- 后：脏标记 + turn 边界（与 entry 批量 fsync 同点）落盘；崩溃恢复靠既有重建路径（manifest 可丢弃）。
- 优劣：正确性不变（缓存可重建），吞吐恢复设计意图；备选维持现状（性能损失常驻）。

## 6. W6 Completions 回放测试

- 前：spec r3308 自称「由插件单测（SSE 回放）覆盖」，openai.rs 无终态矩阵回放测试。
- 后：补双 Done / EOF 无终态 / 中断 flush 三类回放（对齐另两方言与共享校验器断言）。

## 7. W8 注入点两处缺陷

- 前：session_shutdown 仅 config 枚举（config.rs:77），coordinator 不评估——静默失效；session_start 谓词 = step_index==0（coordinator.rs:37-40）→ 每 run 触发而非进程内首次。
- 后：session_shutdown 接线为「下一次 session_start 补记」（对齐已决决策 2）；session_start 改 first_of_session 标志，每 run 语义由 run_start 承担。
- 优劣：补记保留点完整性；备选装载期对 session_shutdown 报错（把负担丢给用户，不取）。

## 8. W9 命令超时杀进程

- 前：run_command_capture 超时返回但子进程残留（host_env.rs:26-75 无 kill_on_drop）。
- 后：kill_on_drop + 超时显式 kill 进程组（对照 external_plugin.rs:96 先例）；r3317「超时中止」兑现。

## 9. checkpoint 解析失败短路

- 前：最新 checkpoint 解析失败（如旧字段 step_id）→ 静默回扫更旧 todo/plan checkpoint 条目（checkpoint.rs:84-94），可能复活陈旧 attempts/steps。
- 后：失败短路返回 None + warn（对齐 r2941「按无 checkpoint 处理」）。
- 优劣：备选 serde(alias) 一行兼容 step_id——违反「不留兼容解析」已决口径，不取。

## 10. 卫生项清单（各 ≤ 半天）

v1 门禁回收（specs 无 v1 字样，allowlist 项删除安全）；golden 夹具生成脚本（兑现 c3233 T1「可重复跑」）；drift stat 快路径（mtime+size 命中跳过重哈希）；context_reminder 行持久化→投影端到端 BDD；T8 fake 示例注入器（三步接入验证）；critic follow-up 变体 + goal/schedule 转移矩阵单测；compaction 快照保留测试补 context-fingerprint-snapshot 类型；decide_active_goal 内部转移走表；勘误记录入 research（BeforeStepCtx 实为 4 字段含 session_id 且落地 spec r3313 一致；c3235 trust 条款落点为 app-tui-trust）。

## 11. W7（待决，定稿后并入）

- ① 改约：design/spec 勘误「提醒行紧随用户消息、首个 step 之前」；零代码；fork 场景提醒丢失、保留双通道。
- ② 修序（推荐）：turn 提交管线在用户消息落盘前调用注入协调器（单调用点，非新中间件阶段）；提醒行进 store + 内存 history；fragment 通道取消；顺序对齐家族契约；fork 安全。

## 12. spec delta（landing 时）

- W2 / W3 / W8 / W9 对应条款已有（实现补齐即可）；C1 补 r2966 一句「write 工具路径同样经表」；W5 无 spec 变化；W7 若 ② 则 agent-prompt r3312 措辞维持「之前」并在场景中固化顺序断言。
