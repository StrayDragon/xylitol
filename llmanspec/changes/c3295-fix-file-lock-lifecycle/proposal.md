---
depends_on: []
title: "Fix file lock lifecycle"
---

# 文件锁生命周期修复

## Why

两处文件锁以「锁文件存在性」为状态、失败路径不清理（静态扫描，实施前复核）：

- **trust 库锁**（A16，trust/store.rs:194、:833）：with_lock 仅非 panic 路径释放——f() panic 即留死锁文件；无 Drop guard；固定 20ms × 10 次无退避。
- **config settings 锁**（C17，config/settings/storage.rs:43-77）：acquire 失败被忽略（is_ok()）仍继续写入；读取发生在加锁之前（读-改-写不互斥）；陈旧锁无超时/PID 校验，崩溃后永久「无锁但自称有锁」。

## What Changes

- 统一锁 RAII guard（Drop 删锁文件）+ 指数退避重试 + 陈旧锁检测（PID/超时）。
- settings 读-改-写全程持锁；获取失败显式失败而非静默无锁写入。

## Capabilities

- domain-security（trust store）；runtime-config（settings 存储）。

## Impact

- 范围：crates/xylitol-runtime/src/trust/store.rs、plugins/xylitol-official-plugin-config/src/settings/storage.rs。
- 验证：panic 注入不留死锁；并发写 settings 不丢更新；陈旧锁自动回收测试。

## Open Questions

1. 陈旧锁判定：PID 存活检查（POSIX）还是 mtime 超时（跨平台简单）？
