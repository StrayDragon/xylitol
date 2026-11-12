# Tasks — 文件锁生命周期修复

> 实施者导读：design §7 是复核结论。收口走 finalize，不列为任务。

- [x] T1 spec delta：domain-security r3344 锁 RAII / r3345 陈旧锁回收；runtime-config r3346 settings 读改写持锁
- [x] T2 SDK file_lock：FileLockGuard + acquire_file_lock（退避 + mtime 陈旧回收 + pid 内容）；单测（panic 注入、陈旧接管）
- [x] T3 trust store：with_lock 改 guard；acquire_lock_at 改调 SDK；单测
- [x] T4 settings：锁内读取 → f → 写 → guard 释放；获取失败 log::error 不写；并发写不丢更新单测；只读不建目录
- [x] T5 回归：just fmt / lint / test / qa；llman-sdd validate c3295 --strict 绿
