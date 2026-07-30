---
id: c1370-update-package-tui-hot-buffer-cap
stage: draft
depends_on:
  - c1360-update-app-tui-tick-local-render
---

# Proposal: TUI 引擎热缓冲封顶（长会话视口滑动）

> **仍 deferred**。**与 c1760**：activity-fold 减行后可能 **缓解** 引擎 `previous_lines` 压力；**不是**被 c1760 吸收实现。超长跑若仍胀，再单独升格。

## Why

差异渲染已把屏外行滚进模拟器 scrollback，但 `previous_lines` 仍随会话线性增长：每帧 O(n) 字符串比对 + 内存膨胀，长时间运行仍会拖垮性能。需要「足够大但有界」的热缓冲，上方内容视为已写入模拟器 scrollback，保留滚轮回看习惯。

## What Changes

1. **`packages/xylitol-tui` 引擎**：为 `previous_lines`（及配套 viewport 索引）设可配置热缓冲上限（建议默认 ≈ `rows * 40`，夹在合理上下界，如 512…8192 行）。
2. **滑动封顶**：超出时丢弃缓冲前缀（不擦模拟器历史）；后续差分配准 `previous_viewport_top`，MUST NOT 因封顶触发无谓全屏清屏。
3. **契约**：上限足够大以保证常见会话流畅；产品面可不改 API 形态（引擎默认开启）。

## Capabilities

- `package-tui-engine`（热缓冲 / 视口）

## Impact

- 长跑 TUI 内存与每帧 diff 成本有界；用户仍可用终端模拟器滚轮回看已滚出内容。
- 非目标：改产品 `UiModel` 条目裁剪；改 ExpandableOutput 块级视口；替代 c1360 Tick 局部刷新。
