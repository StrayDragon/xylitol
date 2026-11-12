---
depends_on: []
title: "Fix compaction boundary and one-shot notices"
branch: sdd/c3290-fix-compaction-boundary-notices
base_branch: main
base_sha: 5e75637997e46f9c1003f1d9dc9682bb208c94a3
---

# 压缩边界推导与一次性通知修复

## Why

压缩插件的状态问题集中在「双份推导」与「一次性标志跨调用方」（静态扫描，实施前复核）：

- **边界双份实现**（B11）：boundary_start 在 prepare_compaction（lib.rs:104-118）与 compact_session（:177-193）各实现一次，对「末条即 Compaction」处理不同（一报错一继续）；last 判定只看最后一条，压缩后追加非消息 entry 即失效。
- **一次性通知先置位后发**（B10）：fallback/floor 两个 &mut bool 标志在 emit 前置位，emit 失败通知永久丢失；状态跨 turn 由调用方维护（runtime 侧 overflow_recovery_attempted，B9）。
- **切点哨兵**（B12）：turn_start_index: isize(-1 哨兵) + is_split_turn 冗余双字段，两处独立解释（lib.rs:130-134 与 :224-228），只改一处越界。
- **压缩后不变量重建**（B14）：四条 ensure_* 各自「重载→检查→追加」，任一失败仅 warn，上下文永久少一条状态且无重试标记。
- rider：角色白名单两份（B18）；0 兼作未知哨兵（B19）。

## What Changes

- 单一 boundary() 推导函数双调用方共用；末条判定规则显式（含非消息 entry 追加场景）。
- 一次性通知改为「发送成功后置位」或带重试的结构（通知栈），失败可观测。
- CutPointResult 哨兵改 Option/显式枚举，单点解释。
- ensure_* 重建合并为单一事务（一次加载、逐项补齐、失败记录待重试标记）。

## Capabilities

- domain-compaction（切点 / 摘要 / token 估计）。

## Impact

- 范围：plugins/xylitol-official-plugin-compaction（lib、orchestrator、cut_detector）。
- 验证：两调用方边界一致性单测；通知失败注入测试；压缩后四类状态齐全断言。

## Open Questions

1. overflow_recovery_attempted 的复位点（runtime react 循环）是否并入本 change 还是留在 c3240？
