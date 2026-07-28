---
change_id: c1660-add-compaction-overflow-retry
title: Context overflow 时 compact-and-retry（对齐 pi）
status: designed
priority: 1660
depends_on:
- c1640-add-compaction-turn-auto-trigger
- c1650-update-compaction-cut-split-turn
author: agent
branch: sdd/c1660-add-compaction-overflow-retry
base_sha: 8d1e8ba37a96b2a4d0d8375c071628224dcedddc
checkpointed: true
checkpoint_sha: 8d1e8ba37a96b2a4d0d8375c071628224dcedddc
---

# c1660-add-compaction-overflow-retry

> **依赖**：c1640、c1650 均已归档。
> **对照**：pi `agent-session.ts` `_checkCompaction` Case1 + `_runAutoCompaction("overflow", willRetry)`；`pi-ai` `isContextOverflow`。
> **设计**：见同目录 `design.md` / `tasks.md`。

## Why

`retry.rs` 称 overflow 交给 compaction，但无 compact-and-retry。pi：overflow → compact →（可）重试一轮；与 threshold 分离；默认仅一次。

## What Changes

- **识别**：稳定 helper（usage 超窗 / length+零输出 / 模式集 + 排除表）；sameModel 闸。
- **编排**：turn-end Case1 先于 threshold；一次 recovery；`willRetry` 仅非 `stop`；摘错 assistant 再续跑。
- **事件**：`reason=overflow`；`CompactionEnd` 补 will_retry / error_message。
- **retry**：同源检测，overflow 不进 AutoRetry。
- **不做**：instructions、TUI%、可配置 max retries、extension compact。

## Capabilities

`domain-compaction` · `agent-runtime`

## Impact

- 超窗错误可自动恢复一轮；二次失败有明确文案。
- `CompactionEnd` 线协议字段扩展（向后兼容默认）。

## Ethics

- risk_level: medium
- prohibited_actions: 无限 compact；误分类 4xx/429；提前实现 c1670/c1680
- required_evidence: overflow-retry-ok / overflow-once / wrong-model / reason-overflow
- refusal_contract: 不做动态压缩策略
- escalation_policy: provider 错误不齐时先统一检测 helper，再考虑 bridge kind
