---
change_id: c1640-add-compaction-turn-auto-trigger
title: Turn 后自动 compaction 与手动 force 路径（对齐 pi）
status: purpose-draft
priority: 1640
depends_on:
  - c1630-update-compaction-reserve-formula
author: agent
---

# c1640-add-compaction-turn-auto-trigger

> **流程**：本 change 仅 `purpose-draft`。**禁止**在未 `change start` / apply 前改 live `spec.toon` / `.feature`。
> **依赖**：须等 **c1630 归档**（reserve 公式 + 无 `compaction_threshold`）后再 promote/apply。
> **对照**：`../pi/packages/coding-agent/src/core/agent-session.ts` — `compact()` / `_checkCompaction`（仅 threshold 支路；overflow 属 c1660）。

## Why

c1630 只换触发尺子。今日 `XyDriver::compact()` → `maybe_auto_compact`（先过闸），且 **turn 结束后无人调用**。pi：回合后 `_checkCompaction`；手动 `compact()` **强制**摘要。须补接线并拆开 auto / force。

## 需求锁定（日后写入 live specs；apply 不得偏移）

### R1 — Auto 挂点（已决）

- System MUST 在 **agent/session 层**、assistant 回合落定后（对齐 pi：settled / 可 `_checkCompaction` 处）调用 reserve 判定（c1630 公式）。
- MUST NOT 仅靠 TUI host 轮询触发。
- `CompactionSettings.enabled == false` 时 MUST NOT auto-compact。
- 用户 abort 的 assistant（若可区分）MUST NOT 触发 threshold auto（对齐 pi `skipAbortedCheck` 默认）。

### R2 — Auto 生命周期

- 触发时 MUST 发 `CompactionStart`，`reason` 含可区分的 threshold 语义（建议字面或结构化：`threshold`；展示文案可含占用说明）。
- 结束后 MUST 发 `CompactionEnd`（成功 / 失败 / 无可压 均诚实；本 change 不要求 overflow `willRetry`）。

### R3 — Manual force（已决）

- `/session-compact`（无参）与 REST compact MUST 走 **force** 路径（`CompactionOrchestrator::compact` 或等价），**MUST NOT** 再调用 `maybe_auto_compact`。
- Force MUST **不过** reserve 闸。
- `prepare` 无内容时 MUST 返回明确错误：
  - 末条已是 compaction → 等价 pi `"Already compacted"`；
  - 会话过小无可摘要 → 等价 `"Nothing to compact (session too small)"`。
- `XyDriver::compact`（或后继 API）语义 = force；若保留 `maybe_auto_compact`，仅供内部 auto。

### R4 — 防抖 / stale usage（已决）

- 紧接一次 compaction 之后，threshold 检查 MUST NOT 使用 **compaction 边界之前** 的 assistant usage/估计再触发（对齐 pi：assistant / usage 时间戳 ≤ 最新 CompactionEntry）。
- 无任何可信 usage/估计时，threshold 支路 MUST NOT 盲目 compact（可 skip）。

### R5 — 非目标（防漂）

| 禁止在本 change | 归属 |
|---|---|
| overflow compact-and-retry / `reason=overflow` | c1660 |
| split-turn 双摘要 / 放开 assistant 切点 | c1650 |
| `/session-compact <instructions>` | c1670 |
| extension 替换整段 compaction | 不做 |
| 恢复 `compaction_threshold` | 禁止 |
| travel LLM 分支摘要 | A01 保留 |

## 验收锚点（promote 时落 `.feature` / 单测）

| id | Given | When | Then |
|---|---|---|---|
| auto-over | enabled、同源估计超 `window-reserve`、turn 刚落定 | 回合结算 | 发生 compact + Start/End（threshold） |
| auto-under | 估计未超闸 | 回合结算 | 不 compact |
| auto-disabled | enabled=false 且用量很高 | 回合结算 | 不 compact |
| manual-force | 用量未超闸但有可摘要历史 | `/session-compact` 或 Driver force | 仍 compact（或明确已无可压错误） |
| manual-wired | — | Driver/slash compact | MUST NOT 走 maybe 闸 |
| stale-guard | 刚写入 CompactionEntry | 立即再 check threshold | MUST NOT 用压缩前 usage 再触发 |

## Capabilities（预期）

`domain-compaction` · `agent-runtime` · `app-tui-commands` / driver · server REST（与 TUI 同 force 语义）

## Impact

- 破坏性：未超闸时手动也会尝试压。
- 文档：architecture「理想 vs 现状」中 turn 后 auto 可标落地（本 change 归档后）。

## Open Questions

- （已决）挂点 = agent/session，非 TUI 轮询。
- （已决）abort 细控：本 change 最小可用（事件诚实）；完整 AbortController 可随 c1660。
- （可延后）`reason` 字符串 vs 枚举上线协议——promote 时与现有 `XyEvent::CompactionStart { reason: String }` 对齐即可。

## Ethics

- risk_level: medium
- prohibited_actions: 只改文档不接线；manual 继续 maybe；提前改 live specs
- required_evidence: 上表锚点至少单测或 BDD 覆盖；Driver 集成一处
- refusal_contract: 不实现 extension 自定义 compaction
- escalation_policy: 与 steer/follow-up 队列顺序不清时先 design 再 apply
