# Tasks — P0 批次复核收尾

> 实施者导读：每项前/后/优劣见 design 对应节；review 证据（file:line）存 research/review-findings.md。收口走 finalize，不列为任务。

- [x] T1 C1：workflow_write 经 task_transition 校验（replan 例外集）+ 绕表拒绝单测
- [x] T2 W1：RunCoordinator transition 单入口 + Cancelling 守卫 + 转移矩阵单测
- [x] T3 W2：goal/schedule 投影容忍过滤（非法事实保留最后合法态 + warn）+ 存量序列单测
- [x] T4 W3：TodoStatus 未知值显式报错回模型 + 单测
- [x] T5 W5：manifest 脏标记 + turn 边界落盘；吞吐对照断言（append 批不再逐条 manifest fsync）
- [x] T6 W6：Completions 终态矩阵 SSE 回放测试（双 Done / EOF / 中断 flush）
- [x] T7 W8：session_shutdown 补记接线 + session_start first_of_session 谓词 + 单测
- [x] T8 W9：run_command_capture kill_on_drop + 超时杀进程组 + 单测
- [x] T9 checkpoint 短路 + warn（旧字段条目不再回扫）+ 单测
- [x] T10 卫生项：v1 门禁回收；golden 生成脚本；drift stat 快路径；context_reminder 端到端 BDD；示例注入器；critic follow-up 变体与转移矩阵单测；compaction 快照测试补全；decide_active_goal 走表；勘误入 research
- [x] T11 W7 ② 修序（已决）：turn 提交管线在用户消息落盘前调用注入协调器（单调用点）；提醒行进 store + 内存 history；fragment 通道删除；顺序断言（提醒 entry 先于用户消息 entry）+ fork 场景测试（分叉后提醒仍在祖先链）+ spec r3312 场景固化顺序
- [x] T12 全量回归 [blocked-by: T1,T2,T3,T4,T5,T6,T7,T8,T9,T10,T11]
  just fmt / lint / test / qa；llman-sdd validate --all --strict；rg 终检（v1 allowlist 已删等）
