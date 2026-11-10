---
depends_on:
  - c3206-refactor-session-graph
title: "Add fork-time context reduction"
---

> **⚠️ deferred（2026-11-10）**：为隐式状态机修复批次（c3230 起）让路，整体移入 delayed-changes；启动时经 propose 迁回 `changes/`。

## Why

分叉时上下文可能很大；provider 前缀缓存是否命中不确定（断点、cache key、TTL、最小 token 都可能不满足）。为了让子分支从可工作的上下文开始，可能需要在 fork 时做 summary / compaction / handoff。

但这些触发的类压缩操作可能失真：摘要丢细节、压缩切点改变语义、handoff 继承状态不完整。初版实现应简化，先支持原始功能（fork + branchSummary），不自动做压缩；后续再把 fork-time 上下文缩减做成可配置、可回滚的机制。

## What Changes

- Fork 策略：none（缺省）/ summary / compaction / handoff。
- summary：从 fork 路径生成 branchSummary；可选模型摘要或启发式摘要；保留来源指针。
- compaction：在 fork 点生成 compaction epoch；明确丢失范围；仅显式启用。
- handoff：写 anchor（读边界）+ 最小继承状态；执行原点移到新 anchor 之后。
- 初版（c3206）：fork 只写 branchSummary，不自动 compaction；保持原始 fork 功能。
- 后续：策略配置、质量证据、回滚、与 provider cache key/断点对齐。

## Scope

- 不在 c3206 初版实现。
- 不做自动改写历史；只追加事实。

## Related

- c3206-refactor-session-graph
- #22（上下文缓存与极致压缩，已关闭）
- #71（决策复用）
- #19（loop 管理，已关闭）

## Open Questions

- 失真度量：摘要覆盖率、来源可追溯、用户确认。
- 触发条件：上下文 token 阈值、fork 深度、用户显式请求。
- 与 provider cache 的关系：fork 后共享 key 还是新 key。
- 回滚：保留原始路径，允许回到未压缩视图。
