---
depends_on:
  - c2070-add-package-tui-dual-interaction-modes
blocks: []
---

# 产品 TUI 固定 Mode B（ath30 B-only）

> **延后提案（2026-08-12）**：自 [`c2070`](../c2070-add-package-tui-dual-interaction-modes/proposal.md) **任务 7.5** 拆出。
> **本 change 不在** `sdd/c2070-…` 分支实现；c2070 收口专注库双入口 / host seam / Mode B 行为质量。
> **状态**：规划草案（Designed 前可充实 tasks；**勿**在 c2070 分支改 live `ath30`）。

> **一句话**：产品 `src/app/tui` **MUST** 仅 ApplicationOwned（Mode B）；改写 `ath30`；删产品「默认 Inline / 用户双模式」叙事；库仍暴露 Inline 为遗留入口。

## Why

战略已拍：产品不维护双模式 UX。今日代码与 live `ath30` 仍写默认 Mode A，与战略冲突。
把产品切默认与 Specs landing **延后**，避免与 c2070 库结构债（双入口抽离、ptim14 可复用 API）抢同一分支带宽，并降低「产品先切 B、库仍是 if 分支」的返工面。

## What Changes

1. **Specs landing**：`app-tui-host` / `ath30` → 产品 MUST 固定 Mode B；废「默认可为 Mode A / 用户可选 Inline」。
2. **产品 host**：`TuiRunOptions`（或等价）默认 `ApplicationOwned`；产品测试 / Fake / harness 跟默认。
3. **文档**：`src/app/tui/AGENTS.md`、相关 PI_DELTAS（如 D16）与产品人验清单对齐 B-only。
4. **非目标**：库删 Inline；在 c2070 未收口前强行产品切默认；fold 点击实现（仍 `c2040`）。

## Capabilities

- `app-tui-host`（ath30 行为改写）
- 按需触及产品 TUI 相关 feature / harness（不扩库 Mode B 合约）

## Impact

| 层 | 影响 |
|---|---|
| live specs | `ath30` 语义翻转（产品默认） |
| `src/app/tui` | 默认交互模式 + 测试期望 |
| 库 `xylitol-tui` | **无**（除非 host 接线暴露缺口，回馈 c2070 已知债） |
| 下游 | 产品可点折叠（`c2040`）的正式 UX 依赖本 change 默认已开 Mode B |

## 依赖

```text
c2070（库 Mode B 基础 + 双入口收口）
  └─ c2071（本 change：产品 ath30 / host 固定 B）
```

**硬前置**：c2070 归档（或至少库 ApplicationOwned 入口 + ptim14 host 清单可复用），再 `change start` 本票。

## Out of scope

- 库双入口结构重构（属 c2070）
- Copied chrome 最终产品落点细节（可与本 change 同批或紧随，不挡 ath30）
- Activity fold / 点三角（`c1760` / `c2040`）

## Open Questions

1. 产品是否保留内部 lab/env 逃生回 Inline（文档化非用户设置）？
2. Dump 产品侧是否暴露 opt-out（依赖 c2070 dump API）？
3. B-only 上线前 Ghostty / tmux / SSH 人验谁签？

## Further Notes

- 原 c2070 task **7.5** 全文迁入本 change；c2070 tasks **已删除 7.5**。
- 调研指针仍在 c2070：`research/keep-or-drop-inline-mode.md`、`research/open-questions-deep-dive-agenda.md`（ath30 措辞）。
