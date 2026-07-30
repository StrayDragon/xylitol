---
change_id: c1755-update-tui-travel-notice-placement
title: Travel / 会话导航通知改为贴底可滚 System 行（前置）
status: purpose-draft
priority: 1755
depends_on: []
blocks:
  - c1760-add-tui-activity-fold
author: agent
---

# Travel / 会话导航通知改为贴底可滚 System 行

> 仅规划、不实现。意向为 **c1760 activity-fold** 的前置。与 c1760 一并继续调研，不急 propose。

## Why

`rebuild_scrollback_from_travel` 在 **entries 最前**插入：

`history @ {selected} · leaf={leaf} · path: …`

跟底时长史下该行常在视口外。fork/resume/import 已用尾随 `push_system_note`；fork 注释已避免顶+底双条。travel 仍是顶插例外。

目标：导航通知改为 **贴底可滚 System**（与其它 note 同族），文案先保持完整 `history @ · leaf · path`。

## 已拍板

| 项 | 决定 |
|---|---|
| 文案 | **先 a**：保留完整 `history @ · leaf · path`，只改放置（顶插 → 尾随） |
| System 族统一 | **后置**：另案统一处理各类 System message（本 change 不扩 scope） |
| 正式化时机 | **先搁置**（继续调研；不与实现挂钩） |

## 调研：同类处理盘点

| 模式 | 落点 | 跟底可见？ | 备注 |
|---|---|---|---|
| travel `history @` | **prepend** | 否（长史） | 本 change 主靶 |
| fork / resume / import / clone / restored | 尾随 `push_system_note` | 是 | 目标形态 |
| queue strip | chrome | 是 | 非 scrollback System 墙（`ati11`） |
| status / Loader | 底栏 | 是 | 短时态 |
| BranchSummary / Compaction | 时间线内 | 随位置 | 非导航 toast |
| slash / abort / Error | 尾随 | 是 | 与 fork note 同族 |

## What Changes（意向）

1. 停止 rebuild **prepend** `history @ …`（产品 + 视需要的 demo）。
2. travel 成功后 **尾随**完整文案的可滚 System。
3. 与 fork/resume note **去重**（不顶+底双条）。
4. harness：通知在 entries **末尾** / 跟底可见；`entries[0]` MUST NOT 再是顶插 `history @`。

## Out of scope

- 统一所有 System 文案/样式（**未来另案**）
- activity-fold（c1760）；delayed 引擎优化
- 改成 queue strip / 永久 chrome

## 与 c1760

叙事前置；`c1760.depends_on` 指向本 id。两案均先搁置，正式化顺序以后再定。

## Open Questions

1. demo 是否必须同期改？
2. 未来「System 统一」案是否一并收口 fork/`switched →` 文案族？

## Ethics

- risk_level: low
- prohibited_actions: 顶+底重复；本 change 顺手大改全部 System 文案
- required_evidence: harness 末尾可见（apply 前）
- escalation_policy: 改 chrome strip 须用户确认
