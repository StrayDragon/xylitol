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

> **流程**：仅 `purpose-draft`；禁止提前 full / 改 live specs。
> **依赖**：c1640（auto 编排 + 防抖）与 c1650（可靠切点）**均归档**后再 apply。
> **对照**：pi `agent-session.ts` `_checkCompaction` overflow 支路 + `_runAutoCompaction("overflow", willRetry)`；**一次** recovery。

## Why

`retry.rs` 称 overflow 交给 compaction，但无 compact-and-retry。pi：overflow → compact →（可）重试一轮；与 threshold 分离。

## 需求锁定

### R1 — 识别 overflow（已决精神）

- MUST 经稳定错误 kind / 桥接映射识别 context overflow（禁止仅靠不稳定英文子串散落匹配作为唯一手段；可有测试夹具注入）。
- MUST 校验 assistant 与 **当前模型** 同 provider+model（对齐 pi `sameModel`）：换模后旧 overflow MUST NOT 触发对新窗的 recovery。

### R2 — 一次 compact-and-retry（已决）

- 默认 **仅一次** overflow recovery（对齐 pi `_overflowRecoveryAttempted`）；二次仍 overflow → MUST 失败并说明，MUST NOT 无限循环。
- `willRetry`：仅当该 assistant 并非已成功 `stop` 完成答案时（对齐 pi：成功超窗可 compact **但不** `continue` 重试）。

### R3 — 生命周期

- MUST `CompactionStart` / `End`，`reason` 可区分 **overflow**（与 c1640 的 threshold / manual 分开）。
- 重试前：错误 assistant MUST NOT 留在将送入重试的上下文（可保留在 session 历史；对齐 pi 从 agent state 摘掉最后错误 assistant）。

### R4 — 与 threshold 共用防抖

- 复用 c1640 stale-after-compaction 规则。
- MUST NOT 把非 overflow 错误吞进 compaction。

### R5 — 非目标

| 禁止 | 归属 |
|---|---|
| 自定义 instructions | c1670 |
| TUI % | c1680 |
| extension 替换 compaction | 不做 |
| 动态阈值 / 热温冷 | roadmap |
| 可配置 max overflow retries（本 change） | 先钉死 = 1；以后另案 |

## 验收锚点

| id | Then |
|---|---|
| overflow-retry-ok | 注入 overflow → compact → 重试成功（Fake） |
| overflow-once | 第二次 overflow → 失败文案，不再循环 |
| wrong-model | 旧模型 overflow 在换模后不触发 |
| reason-overflow | 事件 reason 可区分 overflow |

## Capabilities

`domain-compaction` · `agent-runtime` · 必要时 `package-ai-bridge`（overflow kind）

## Open Questions

- （已决）重试次数 = **1**。
- Fake 注入方式：promote 时定（错误 kind vs 专用测试钩）。

## Ethics

- risk_level: medium
- prohibited_actions: 无限 compact；误分类 4xx；提前改 live specs
- required_evidence: 上表集成/BDD
- refusal_contract: 不做动态压缩策略
- escalation_policy: provider 错误不齐时先统一 bridge kind
