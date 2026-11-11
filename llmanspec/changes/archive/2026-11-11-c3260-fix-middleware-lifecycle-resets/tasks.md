# Tasks — 中间件生命周期复位与自动化转移

> 实施者导读：先过 design §0 上游 gate；§2 转移矩阵是验收基准。收口走 finalize，不列为任务。

- [x] T1 spec 先行（转移合法性 / turn 边界复位 / 闸门可重试条款）
- [x] T2 critic turn 边界复位（BeforeStep && has_new_user_messages；单一 reset 入口）[blocked-by: T1]
- [x] T3 todo 闸门先判后置 + 失败回滚 [blocked-by: T1]
- [x] T4 goal / schedule 转移表 + 存量容忍投影 + Exhausted 事实化 [blocked-by: T1]
- [x] T5 todo reminder 单 Mutex 快照 + TodoStatus 解析 [blocked-by: T1]
- [x] T6 全量回归 [blocked-by: T2, T3, T4, T5]
  转移矩阵单测；critic 两问 BDD（含 follow-up 变体）；todo 恢复单测；just fmt / lint / test；validate --all --strict。
