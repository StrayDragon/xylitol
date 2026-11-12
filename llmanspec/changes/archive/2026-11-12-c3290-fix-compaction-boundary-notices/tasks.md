# Tasks — 压缩边界推导与一次性通知修复

> 实施者导读：design §7 是复核结论（0 哨兵不成立；角色白名单语义不同不合并；emit 无 Result 故无重试循环）。收口走 finalize，不列为任务。

- [x] T1 spec delta：domain-compaction r3340 边界单点、r3341 通知 emit 后置位、r3342 切点显式 Option、r3343 压缩后单事务补齐
- [x] T2 边界：抽出 boundary_start + last_context_entry 判定；prepare/compact 共用；单测（两调用方一致、非消息尾）
- [x] T3 通知：take_fallback_notice 不置位（返回 PendingNotice）；两处调用方 emit 后置位；floor 标志同步后置
- [x] T4 切点：CutPointResult split_turn: Option<usize>；两处解释与 BDD step 改 if let / is_some
- [x] T5 重建：ensure_state_after_compact 单次加载逐项补齐 + 失败日志 + bool；单测
- [x] T6 rider：design §7 记录语义差异，不改代码
- [x] T7 回归：just fmt / lint / test / qa；llman-sdd validate c3290 --strict 绿
