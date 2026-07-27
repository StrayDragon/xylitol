---
change_id: c1660-add-compaction-overflow-retry
title: Context overflow 时 compact-and-retry（对齐 pi）
status: purpose-draft
priority: 1660
depends_on:
  - c1640-add-compaction-turn-auto-trigger
  - c1650-update-compaction-cut-split-turn
author: agent
---

# c1660-add-compaction-overflow-retry

## Why

pi 在模型报 context overflow 时走 `_runAutoCompaction("overflow")`，并可在压缩后重试该轮；阈值触发与 overflow 恢复分离。xylitol `retry.rs` 注释写「overflow 由 compaction 处理」，但缺少 compact-and-retry 路径——长会话会硬失败而非恢复。

## What Changes

- 识别 provider/桥接层的 context overflow（与现有非 retryable 分类衔接）。
- Overflow → `CompactionStart(reason=overflow)` → compact → 在策略允许下 **重试一轮**（`willRetry` 语义对齐 pi）；失败则诚实结束并说明。
- 与阈值 auto（c1640）共用编排，但 reason / 遥测区分 `threshold` vs `overflow`。
- 压缩后 stale usage 不得立刻误触发（与 c1640 防抖共用）。
- 可选：abort 进行中的 auto-compaction。
- **本 change 不做**：自定义 instructions（c1670）、TUI %、extension 替换。

## Capabilities

| Capability | 变更 |
|---|---|
| `domain-compaction` | overflow 恢复 MUST |
| `agent-runtime` | overflow 与 retry/compaction 编排顺序 |
| `package-ai-bridge`（若需） | overflow 错误可识别映射 |

## Impact

- **破坏性**：原先 overflow 直接失败的路径变为「先压再试」；调用方需能处理中间 Compaction 事件。
- **默认体验**：接近窗顶时更不易裸崩。
- **非目标**：无限重试；把所有 4xx 当 overflow。

## Depends / 后续

```text
c1640 + c1650 ──► c1660 (本)
```

## Open Questions

- 重试次数：对齐 pi「一次 compact-and-retry」还是可配置？（倾向：先一次，与 pi 文档一致。）
- Fake provider 如何注入 overflow 以测恢复。

## Ethics

- risk_level: medium
- prohibited_actions: 把非 overflow 错误吞进 compaction；无限 compact 循环
- required_evidence: 集成/BDD：模拟 overflow → compact → 成功重试；二次 overflow 失败路径
- refusal_contract: 不在本 change 做动态阈值/热温冷策略
- escalation_policy: 若各 provider 错误字符串不一致，先统一 bridge 错误 kind 再挂恢复
