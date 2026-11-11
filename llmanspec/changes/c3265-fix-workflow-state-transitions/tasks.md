# Tasks — Workflow 任务状态转移

> 实施者导读：先过 design §0 上游 gate 与 §1 转移图；§3 更名清单一步到位。收口走 finalize，不列为任务。

- [ ] T1 spec 改写（Task 词汇 + 转移合法性 + 容器阶段）
- [ ] T2 Step → Task 更名清扫（sdk / 插件 / BDD；不留别名；旧 CustomEntry 字段名不迁移）[blocked-by: T1]
- [ ] T3 task_transition 转移表（全写入点接线；非法拒绝；存量容忍投影）[blocked-by: T2]
- [ ] T4 WorkflowPhase 容器阶段机（九字段降级；mark_failed 合一；approved 重跑清位）[blocked-by: T2]
- [ ] T5 全量回归 [blocked-by: T3, T4]
  转移全矩阵单测；重规划 BDD；非法写入拒绝 BDD；行为快照对照；just fmt / lint / test；validate --all --strict。
