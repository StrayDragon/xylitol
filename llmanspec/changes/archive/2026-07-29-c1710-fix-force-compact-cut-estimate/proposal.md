---
change_id: c1710-fix-force-compact-cut-estimate
title: 对齐 pi：compact 用 leaf 分支 + 切点计量，修复误报 Nothing to compact
status: designed
priority: 1710
depends_on: []
author: agent
branch: sdd/c1710-fix-force-compact-cut-estimate
base_sha: 785db08a3bb45d7c7f7c8605587c17a7ea58b0aa
checkpointed: true
checkpoint_sha: 785db08a3bb45d7c7f7c8605587c17a7ea58b0aa
---

# c1710-fix-force-compact-cut-estimate

> **产品口**：对齐 pi，**不**做 force `keep_recent=0`。
> **现象**：footer Api 已数万 tokens 时 `/session-compact` 仍报 `Nothing to compact (session too small)`。
> **对照**：pi `getBranch()` + `estimateTokens(AgentMessage)` + 同一 `prepareCompaction` 闸。

## Why

手动 force 已绕过 reserve 触发闸，但仍走 prepare。切点用盘上 JSON `len/4`（`estimate_tokens_entry`），且 compact 路径吃 `load_entries`（整文件）而非 leaf `getBranch`。结果：footer 跟 Api（含 system/tools）显示「很大」，切点启发式却把整段算进 `keepRecentTokens`（默认 20k）→ 误报 too small。pi 在同类会话上常可 `/compact`，根因是实现差而非「force 应无视 prepare」。

## What Changes

- **条目范围**：prepare / compact / force|auto|overflow MUST 使用与 pi `getBranch()` 等价的 **当前 leaf 分支路径**（`get_branch_entries` 或 port 扩展），MUST NOT 以整文件线性 `load` 作为 compact 唯一输入（分支会话尤甚）。
- **切点计量**：`find_cut_point` 累计 MUST 对齐 pi `estimateTokens`：对上下文可见 `AgentMessage`（含 thinking / toolCall / toolResult / bash 等）做 chars/4；零贡献条目跳过；MUST NOT 以原始 JSON 整包 `len/4` 作为切点 SSOT。
- **错误语义（修订 c17）**：`Nothing to compact (session too small)` MUST 仅在分支路径上确实无可摘要历史时返回；`Already compacted` 保留。MUST NOT 因切点低估 + 默认 keepRecent 把「footer 已很大」的会话误判为 too small（以对齐 pi 的计量为准）。
- **不做**：force `keep_recent=0`；改 reserve 触发公式；改 TUI 文案专案（可附带更清晰 note，非必须）。

## Capabilities

`domain-compaction`（主）· 必要时 `agent-session` / session port（暴露 branch load）

## Impact

- `/session-compact` 在消息体按 pi 尺子超过 keepRecent 时应能压；真短会话仍拒。
- 分支会话 compact 不再误含旁支条目。

## Ethics

- risk_level: medium（改变何时能压）
- prohibited_actions: 默认 keep_recent=0；用 Api footer 直接当切点预算而不对齐 pi；破坏 Already compacted
- required_evidence: 分支路径用例；切点计量与 pi 同构样例；c17 feature 更新；Already compacted 仍红
- refusal_contract: 不做「footer>keep 就强制压」的非 pi 捷径（除非另开 change）
- escalation_policy: 若对齐后仍系统性低估，再开 follow-up 讨论 keep 策略
