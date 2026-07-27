---
change_id: c1630-update-compaction-reserve-formula
title: Compaction 触发改用 pi 同款 reserve 公式并移除百分比闸
status: designed
priority: 1630
depends_on: []
author: agent
branch: sdd/c1630-update-compaction-reserve-formula
base_sha: 8394fd101a7d77ad3496e69ce9c53e42c01320c6
checkpointed: false
---

# c1630-update-compaction-reserve-formula

## Why

xylitol 配置面已有与 pi 对齐的 `compaction.{enabled,reserveTokens,keepRecentTokens}`，但运行时仍用旁路 `compaction_threshold: 0.8`（`usage_ratio >= threshold`）决定是否压缩。这与 pi 心智不一致，也让 `reserveTokens` 对触发几乎无效（仅影响摘要 max_tokens）。要对齐 pi、并为后续 TUI「百分比仅作派生展示」打底，必须先把触发 SSOT 改成 reserve 公式并删除百分比闸。

## What Changes

- **触发公式**（与 pi 一致）：`contextTokens > contextWindow - reserveTokens`（`enabled=false` 时永不触发）。
- **删除** `compaction_threshold: f64` 及装配路径（builder / composition / bootstrap / `AgentCapabilities` / `should_compact(..., f64)`）。
- **单一配置 SSOT**：仅 `XyCompactionSettingsConfig` / 运行时 `CompactionSettings`（`enabled` / `reserve_tokens` / `keep_recent_tokens`）；默认 `16384` / `20000`。
- `get_context_usage` / `ContextUsage.should_compact`：改吃 `CompactionSettings`（或 reserve），**percent 字段可保留为派生展示**（非触发 SSOT；TUI % 条见 c1680）。
- 示例 / schema / architecture 文档：去掉「默认约 80%」叙事；改为 reserve 心智。
- 更新 `domain-compaction` / `runtime-config` 合约与 BDD：c2/c16、`need-compact`/`no-compact`、rc15 映射断言。
- **本 change 不做**：turn 后自动接线（c1640）、split-turn（c1650）、overflow retry（c1660）、slash instructions（c1670）、TUI % 条（c1680）。

## Capabilities

| Capability | 变更 |
|---|---|
| `domain-compaction` | 修订 c2/c14/c16；删百分比闸；运行时类型名对齐 `CompactionSettings` |
| `runtime-config` | rc15 映射验收改为 keepRecent/reserve，禁止 threshold 字段 |
| `docs/architecture` | 压缩与上下文：默认心智对齐 pi |

## Impact

- **破坏性**：依赖「约 80% 触发」的测试/脚本需改；`compaction_threshold` API 消失。
- **默认体验**：默认 `reserveTokens=16384` 时，相对 128k 窗约在 ~87.5% 处触发——**不是**刻意保留 80%，而是跟 pi。
- **非目标**：auto 接线、算法加深、TUI 百分比展示。

## Depends / 后续

```text
c1630 (本 · 本分支 full) ──┬──► c1640 auto/manual 接线（purpose-draft，勿提前 full）
                          ├──► c1650 cut + split-turn（purpose-draft）
                          ├──► c1660 overflow（依赖 1640+1650）
                          ├──► c1670 optional instructions（依赖 1640）
                          └──► c1680 TUI %（后置 draft）
```

> 兄弟 change 仅 draft 锁需求；**禁止**在未 `change start`+apply 前改其 live specs。需求正文以各 `changes/c16xx-*/proposal.md`「需求锁定」为准。

## Open Questions

- （已决）放弃百分比触发；百分比仅可后置为派生 UI。
- （已决）配置字段名保持 camelCase，与现有 YAML/pi 一致。
- （已决）`ContextUsage.percent` 可保留；`should_compact` 必须走 reserve。

## Ethics

- risk_level: medium
- prohibited_actions: 保留第二套百分比触发 SSOT；静默把 footer 百分比当触发闸
- required_evidence: 单测/BDD 覆盖 reserve 公式边界与 enabled=false；全仓无 `compaction_threshold` 残留；rc15 映射绿
- refusal_contract: 不在本 change 引入动态/按段策略（roadmap 极致压缩）；不实现 turn 后 auto 接线
- escalation_policy: 若默认 reserve 相对旧 0.8 触发更晚/更早引发产品异议，先钉默认值再 apply
