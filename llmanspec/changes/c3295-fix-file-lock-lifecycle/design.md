# Design — 文件锁生命周期

## 1. 目标与边界

- 两处文件锁统一 RAII + 退避 + 陈旧检测；panic 不留死锁；settings 读-改-写互斥。
- 边界：锁文件名与目录布局不变。

## 2. 决策摘要

| 决策 | 内容 | 状态 |
|---|---|---|
| RAII | FileLockGuard（Drop 删锁文件）；with_lock / settings 写路径改持 guard | 定稿方向 |
| 退避 | 指数退避（20ms 起 ×1.5，上限 10 次）替代固定间隔 | 定稿方向 |
| 陈旧检测 | mtime 超时（默认 30s，可配置）+ 锁内容写 pid/主机名辅助 | 推荐，待 review |
| settings 互斥 | 读取纳入锁内（读-改-写全程持锁）；获取失败显式报错 | 定稿方向 |

## 3. 测试边界（seam）

- 单测：panic 注入（catch_unwind）后锁文件已清；并发写不丢更新；陈旧锁自动接管。
- BDD：runtime-config settings 写入回归。

## 4. 风险与回滚

- mtime 超时误判长持锁进程——阈值保守并允许配置；trust / settings 两片独立回滚。
