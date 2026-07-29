---
change_id: c1755-update-tui-travel-notice-placement
title: Scrollback 导航/瞬时通知一律贴底可滚（禁顶插）
status: designed
priority: 1755
depends_on: []
blocks:
- c1760-add-tui-activity-fold
author: agent
branch: sdd/c1755-update-tui-travel-notice-placement
base_sha: 096ef094cfed1ab7b504b854939b7f7b43d5a0c7
checkpointed: true
checkpoint_sha: 096ef094cfed1ab7b504b854939b7f7b43d5a0c7
---

# c1755 — Scrollback 通知贴底可滚（禁顶插）

> 由 travel `history @` 前置扩大为：**凡重建/导航类瞬时 System 通知，MUST 贴底可滚（输入框上方），MUST NOT 顶插进 entries 前缀。**
> 产品 + demo 同 change。前置于 c1760 activity-fold。

## Why

跟底（follow-bottom）时，插在 `entries[0]` 的通知常在视口外，等于没提示。fork/resume/import 等已用 `push_system_note` **尾随**；travel 重建仍 **prepend** `history @ …`，是明确例外。

代码审计（2026-07-30）：产品路径上 **唯一系统性顶插违例** 即 `rebuild_scrollback_from_travel` 的 `history @`；slash/错误/会话切换 note 已尾随。扩大范围的意义是钉死 **政策 + 回归闸**，避免以后再开顶插，并同步 demo。

## 已拍板

| 项 | 决定 |
|---|---|
| 文案 | travel 仍用完整 `history @ · leaf · path`，只改放置 |
| 政策 | 导航/瞬时 scrollback 通知 → **尾随**；**禁止**为「让用户看见」而 prepend |
| 与已有尾随 note | fork/`switched →` 等路径 **不再叠** `history @`（去重） |
| demo | **同 change** 改为 path 后再尾随 System |
| System 文案族统一 | **另案**（本 change 不改 fork/switched 文案内容） |

## What Changes

1. `rebuild_scrollback_from_travel`：**只**投影祖先路径条目，**不再**插入任何 System banner。
2. `apply_session_tree_travel`：重建后 `push_system_note(history @ …)`。
3. `apply_session_tree_fork` / `apply_switched_session` / `apply_debug_scene`：保持既有尾随 note；**不**再因 rebuild 带出 `history @`。
4. `agent_demo` travel：先重建 path，再尾随 `history @`（与产品同形）。
5. harness / demo 测：跟底可见；`entries` 首条 MUST NOT 为顶插 `history @`。
6. live specs：session-tree + transcript 政策句；feature 场景可执行或文档场景按 Partitioned 规则。

## Capabilities

| capability | 角色 |
|---|---|
| `app-tui-session-tree` | travel 通知放置；demo travel 同源 |
| `app-tui-transcript` | 重建路径禁止顶插瞬时导航通知（政策） |

## Impact

- Travel 后用户立刻看到 `history @`；长史顶不再无效噪声。
- 为 c1760 清场（折叠规则不必特判「看不见的顶栏」）。
- fork/resume 底栏仍一条产品 note，无双 System。

## Out of scope

- 统一所有 System 文案/样式
- activity-fold（c1760）；queue strip / status chrome 形态
- 改 session JSONL；把 BranchSummary 等**时间线内容**当 toast（它们按路径投影，不是顶插 toast）

## Ethics

- risk_level: low
- prohibited_actions: 顶+底重复同一导航通知；本 change 顺手大改全部 System 文案
- required_evidence: harness + demo 断言贴底可见
- escalation_policy: 若改成 chrome strip 须用户确认
