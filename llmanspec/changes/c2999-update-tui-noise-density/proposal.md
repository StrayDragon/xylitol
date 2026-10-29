---
depends_on: []
title: TUI 减噪与对比度：队列条压缩与次要文本对比度下限
needs_specs_change: true
branch: sdd/c2999-update-tui-noise-density
base_branch: main
base_sha: 0b741bec08ad9840545391158e596aa0d5d9b547
---

# TUI 减噪与对比度：队列条压缩与次要文本对比度下限

## Why

- #25 的 M2 指出忙碌 / 队列 / 中止等状态表达重复、多徽章抢视线；当前 queue strip 每条排队消息各占一行，再叠加一条 Alt+Up 提示，队列一多就把固定区撑高。
- #25 的 M3 指出 DESIGN 对比度 / 密度还可打磨；当前暗色 `muted`（`#6c7086`）在默认 `surface`（`#1e1e2e`）上的对比度约 3.36:1，低于常用 4.3:1 的次要文本可读下限。
- 两项都不改变语义：队列仍可见、可 Alt+Up 还原；色板仍只映射既有语义 token。

## What Changes

- Queue strip 压缩：
  - steer / follow-up 每类 至多一行；有排队消息时显示该类的**最新**一条正文。
  - 该类消息数 > 1 时在标签后附计数，如 `Steering (2): <latest>`；仅 1 条时不显示计数，保持 `Steering: <msg>`。
  - 保留 `↳ Alt+Up to edit all queued messages` 提示与 muted 色。
  - 不再逐条铺开所有排队消息，避免固定区随队列长度线性增高。
- 次要文本对比度：
  - 暗色 `muted` 由 `#6c7086` 调整为 `#7f849c`（在 `surface` 上约 4.44:1）。
  - DESIGN.md frontmatter 与 `Palette::dark()` 同步；`tool` / `diff-context` 保持更暗，维持「工具摘要比 status/footer 更弱」的层级。
  - 新增产品规则：默认暗色次要文本对比度 须 ≥ 4.3:1。
- 规格：
  - `app-tui-input` 的 `product-queue-fixed-zone` 规则补充「每类至多一行 / 最新正文 / >1 才显示计数」。
  - `app-tui-fixed-zone` 新增次要文本对比度规则。

## Capabilities

- `app-tui-input`（queue strip 压缩）
- `app-tui-fixed-zone`（次要文本对比度下限）

## Impact

- **用户可感知**：队列多时固定区不再逐条增高；status/footer/边框等次要文本更易读。
- **不变**：队列内容仍可见、Alt+Up 仍可还原全部；不新增 footer/status 队列徽章；不改色板语义。
- **可回退**：恢复逐条渲染与旧 `muted` hex 即可。

## Non-Goals

- 不实现密度三档（紧凑 / 标准 / 宽松）。
- 不做色弱 / 高对比预设。
- 不改忙碌 status 词汇、abort 流程或固定区顺序。
- 不把队列计数放进 footer / status。
