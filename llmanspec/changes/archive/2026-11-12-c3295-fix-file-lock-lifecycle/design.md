# Design — 文件锁生命周期（2026-11-13 复核后定稿）

## 1. 目标与边界

- 两处文件锁统一 RAII + 退避 + 陈旧检测；panic 不留死锁；settings 读-改-写互斥。
- 边界：锁文件名与目录布局不变。

## 2. 决策摘要（2026-11-13 复核后全部定稿）

| 决策 | 内容 | 状态 |
|---|---|---|
| RAII | 统一实现落 plugin-sdk 新模块 file_lock：FileLockGuard（Drop 删锁文件）；trust with_lock 与 settings 写路径改持 guard | 定稿（修正：两 crate 共享需落 SDK，不各写一份） |
| 退避 | 指数退避 20ms 起 ×1.5，上限 10 次（替代固定间隔） | 定稿 |
| 陈旧检测 | mtime 超时（默认 30s，可经参数覆盖；锁内容写 pid 辅助诊断），超时后删除旧锁并重试 | 定稿（Open Q1 取 mtime：跨平台、无 pid 存活探测依赖） |
| settings 互斥 | 锁内读取 → f() → 写 → Drop 释放；获取失败 log::error 且不写入（trait 返回 ()，无 Result 可返） | 定稿（修正：SettingsStorage trait 无 Result，失败以日志显式化） |

## 3. 锁协议

acquire(path, stale_after)：create_new 写 pid；AlreadyExists → 若 mtime 超 stale_after 则 remove 后重试；否则指数退避；10 次失败返回 Lock 错误。FileLockGuard::release 显式删文件；Drop 兜底删。

## 4. 测试边界（seam）

- 单测：panic 注入（catch_unwind）后锁文件已清；陈旧锁自动接管；并发写 settings 不丢更新；获取失败不写入。
- BDD：runtime-config settings 写入回归。

## 5. 风险与回滚

- mtime 超时误判长持锁进程——阈值保守；trust / settings 两片独立回滚。

## 7. 实施前复核（2026-11-13）

| 结论 | 复核 | 证据 |
|---|---|---|
| trust with_lock 仅非 panic 释放、无 Drop guard | 成立 | trust/store.rs:194-200 with_lock；release_lock 手动 remove |
| trust 固定 20ms × 10 无退避、无陈旧检测 | 成立 | trust/store.rs:833-870 acquire_lock_at |
| settings 读在加锁前、acquire 失败仍写 | 成立 | settings/storage.rs:88-110 with_lock |
| settings 陈旧锁无超时 | 成立 | storage.rs:43-77 acquire_with_retry |
