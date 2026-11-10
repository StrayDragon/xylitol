---
depends_on:
  - c3100-add-agent-preset-resume-diff
title: "Add agent profile resume choice and re-injection"
---

> **⚠️ deferred（2026-11-10）**：为隐式状态机修复批次（c3230 起）让路，整体移入 delayed-changes；启动时经 propose 迁回 `changes/`。

## Why

c3100 已记录 session header 的 profile name + fingerprint，并在 resume 时比较、记录 mismatch notice；但未接 TUI 三选项 Choice、未做全量重新注入、未做 per-session profile binding、profile 缺失 fallback 与审计。

## What Changes

- resume 检测 profile fingerprint 变化后，TUI 弹 Choice：
  1. Compact 后继续（推荐）：先 compaction，再全量重新注入上下文。
  2. 不 compaction 继续：全量重新注入上下文；风险由用户承担。
  3. 取消 resume：保持 session 不变。
- 全量重新注入：粗粒度策略，不做 diff / delta。
- per-session profile binding：agent assembly 按 session 的 profile 构建，不用 host boot profile 替代。
- profile 不存在：回退默认 profile 并提示；不得静默用错。
- 审计：记录 profile changed / fallback 决策。
- TUI 通知：resume 时显示 profile changed / fallback。

## Scope

- 推荐 mid-session 不改 profile；变更只发生在 resume。
- 不做 diff / delta 重注入。
- 复用 c3100 的 header metadata 与 fingerprint。

## 延后原因

- 需要 runtime assembly 按 session 重建、compaction API、TUI Choice 与审计，跨 crate。
- profile fallback 策略与全量重注入的成本/上下文窗口处理未定。
- 主线先交付 header 记录与比较 notice。

## 启动前置条件

- c3100 已归档（当前已满足）。
- 确定 profile 缺失 fallback 策略与提示。
- compaction API 稳定，且可在 resume 前触发。
- TUI Choice 槽位与 resume 流程可插入。
- 确定审计事件字段与落点。
- 评估全量重注入的 token 成本与上限。

## Related

- c3100-add-agent-preset-resume-diff（已归档）
- c3060-add-mcp-tool-catalog（resume tool set）
- c3206-refactor-session-graph（graph head / build_context）

## Open Questions

- profile 缺失时回退默认还是拒绝 resume。
- 全量重注入是否自动触发 compaction。
- mid-session profile 变更是否允许。
- 审计保留周期与脱敏。
