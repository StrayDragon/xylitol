---
depends_on:
  - c2070-add-package-tui-dual-interaction-modes
blocks: []
---

# 产品 TUI 固定 ApplicationOwned（ath30 AO-only）

> **延后提案（2026-08-12）**：自 [`c2070`](../c2070-add-package-tui-dual-interaction-modes/proposal.md) **任务 7.5** 拆出。
> **本 change 不在** `sdd/c2070-…` 分支实现默认翻转；c2070 收口库双入口 / 启动绑定 / 禁热切。
> **状态**：规划草案（Designed 前可充实 tasks）。

> **一句话**：产品 `src/app/tui` **MUST** 仅 ApplicationOwned；改写 `ath30` 默认；删产品「默认 Inline」叙事；库仍暴露 Inline 为遗留入口。**无**运行中热切（c2070 已移除 `apply_interaction_mode`）。

## Why

战略已拍：产品不维护双模式 UX；退出 AO 后靠 dump 保留主屏可翻内容（类似 inline 留屏），不是切回 Inline。
今日 live `ath30` 仍写默认 Inline（c2070 已钉：启动选定、禁止热切）。把**默认翻成 ApplicationOwned**延后到本票，避免与 c2070 库收口抢带宽。

## What Changes

1. **Specs landing**：`ath30` → 产品 MUST 固定 ApplicationOwned（启动构造）；废「缺省 Inline」。
2. **产品 host**：`TuiRunOptions` 默认 `ApplicationOwned`；产品测试 / Fake / harness 跟默认。
3. **文档**：`src/app/tui/AGENTS.md`、相关 PI_DELTAS（如 D16）与产品人验清单对齐 AO-only。
4. **非目标**：库删 Inline；恢复 mid-session 换模式 API；fold 点击（仍 fold 族）。

## Capabilities

- `app-tui-host`（ath30 默认翻转）
- 按需触及产品 TUI 相关 feature / harness（不扩库双模式合约）

## Impact

| 层 | 影响 |
|---|---|
| live specs | `ath30` 缺省从 Inline → ApplicationOwned |
| `src/app/tui` | `TuiRunOptions` 默认 + 测试期望 |
| 库 `xylitol-tui` | **无**（构造入口已在 c2070） |
| 下游 | 产品可点折叠的正式 UX 依赖本 change 默认已开 ApplicationOwned |

## 依赖

```text
c2070（库 ApplicationOwned + 启动绑定模式 + 禁止热切）
  └─ c2071（本 change：产品 ath30 默认 AO）
```

**硬前置**：c2070 归档（或至少 ptim14 host 清单可复用），再 `change start` 本票。

## Out of scope

- 库双入口结构重构（属 c2070）
- Mid-session Inline↔AO 热切（**明确不做**；c2070 已删产品换栈 API）
- Copied chrome 最终产品落点细节（可与本 change 同批或紧随）
- Activity fold / 点三角（`c1760` / `c2040`）

## Open Questions

1. 产品是否保留内部 lab/env 逃生回 Inline（文档化非用户设置；若保留仍须**启动期**绑定，非热切）？
2. Dump 产品侧是否暴露 opt-out（依赖 c2070 dump API）？
3. AO-only 上线前 Ghostty / tmux / SSH 人验谁签？

## Further Notes

- 原 c2070 task **7.5** 全文迁入本 change；c2070 tasks **已删除 7.5**。
- 调研指针仍在 c2070：`research/keep-or-drop-inline-mode.md`、`_HANDOFF.md`（产品决策：无 live switch）。
- c2070 已落地：`HostSession::new_product_ui_with_meta_mode`；**勿**再引入 `apply_interaction_mode`。
