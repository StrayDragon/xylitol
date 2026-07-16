---
change_id: c1210-fix-package-tui-resize-diff
title: "差分 TUI：终端长宽变化时 viewport 稳定（残影/错位）"
status: purpose-draft
priority: 1210
apply_band: P9-deferred
depends_on: []
author: agent
track: R
wave: tui-engine
domain: package-tui
---

# c1210-fix-package-tui-resize-diff

## Why

产品 TUI（尤其 c1135 启动卡片 + scrollback/editor/footer 栈）在 **终端宽高变化** 时，editor 边框通常仍对齐，但 **上方内容会变脆弱**：残影、行错位、卡片/分子式碎片渗进 editor 与 footer 之间。应用层已对 startup card 做 exact-width pad，仍不足以覆盖所有差分路径。根因更可能在 **`packages/xylitol-tui` 差分引擎 / resize 清屏策略**，而不是单一产品 widget。

## Purpose（**暂缓 / deferred · 低优先级**）

本变更 **P9 暂缓**。升格前先 **稳定复现**，再在 `xylitol-tui` 内修，避免产品层继续打补丁掩盖引擎问题。

### 背景（观察到的症状）

- 拖动终端宽高后：上方 slot（loaded-resources / scrollback）出现错位或「幽灵」行；footer 附近偶发混入上方 ASCII 碎片。
- 对比：editor 输入框对 resize 相对稳健（每帧按宽重算边框）。
- 引擎已有 width-change 时的清屏路径（`previous_width` / force render）；仍不足以覆盖 **高度变化、行数增减、ANSI 截断后 SGR 状态、未 pad 到全宽的差分残留** 等组合。

### 预期回头路径

1. **复现**：用 `TestTerminal` / PTY harness 固定序列（宽↔窄、高↔矮、卡片行数变化）录帧；对照产品 `cargo run` 手拖窗口。
2. **定位**：`packages/xylitol-tui` 的 `do_render` / diff / `request_render(force)` / height shrink 时未擦除的尾部行；必要时查 ANSI `truncate_to_width` 截断是否破坏 SGR。
3. **修复（升格后）**：优先引擎不变量——例如 height shrink 强制擦除多余行、width change 全量重绘、可选「每行 pad 到终端宽」引擎侧保证；产品 widget 只保留契约测试。
4. **验收**：resize 序列后无残影、无行宽 invariant 误报、editor 与上方 slot 同时稳定。

## What Changes（升格 full 时）

- `packages/xylitol-tui`：resize / viewport 差分稳定性（具体方案复现后钉 design）
- 包层回归测试：宽高往返帧断言（无幽灵行）
- 可选：产品 harness 加一条 resize smoke（不替代包层）
- delta：`package-tui-*`（引擎相关 capability；升格时命名）

## Capabilities（预期）

- `package-tui-engine` 或既有差分/render capability（升格时对齐现有 spec 名）
- 视需要：`package-tui-testing`

## Out of scope

- 本波实现（deferred）
- 仅靠 app/tui widget 继续堆 pad/截断补丁当作「修完」
- 改 Codex 卡片视觉（属 c1135）

## Ethics

- risk_level: medium（渲染正确性；误修易引入 flicker 或全量重绘性能回退）
- prohibited_actions: 静默吞掉 width-invariant 错误；用 sleep/重试掩盖 flaky
- required_evidence: 可重复的包层测试 + 至少一条产品手测/resize 录帧
- escalation_policy: 若根因在产品 Component 未遵守行宽合约，先补 widget 再决定是否仍要引擎加固

## Depends

- []（可与 c1135 并行观察；不阻塞 c1135 归档）

## Notes

- 2026-07-16：c1135 Codex 卡片落地后用户仍复现 resize 不稳；应用层 exact-width pad 已做，本草案专责 **xylitol-tui 引擎侧** 跟进。
- 复现提示：启动后拖窄→拖宽、再压矮→拉高；观察卡片与 footer 之间是否残留晶体/边框碎片。
