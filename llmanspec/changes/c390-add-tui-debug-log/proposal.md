---
change_id: c390-add-tui-debug-log
title: 为 TUI 添加 debug 日志管线（文件日志 MVP）
status: draft
priority: 390
depends_on: []
author: agent
---

# c390-add-tui-debug-log

> **状态: draft（调研占位）** — 方案未定，待进一步调研后细化 design 与 tasks。

## Why

TUI inline 模式（`Viewport::Inline`）下，stdout 被 ratatui 完全占据，stderr 会破坏 buffer 渲染。目前 TUI 层零日志输出：没有 `tracing` 调用，没有 `eprintln`，唯一的观测手段是肉眼看 UI 行为。

业务复杂后（多轮 tool call、stream 取消、mpsc 竞态），肉眼观测不可持续。需要一条**不干扰渲染**的 debug 管线。

## 候选方案

| 方案 | 复杂度 | 适合? |
|---|---|---|
| 文件日志（tracing-appender） | 低 | ✅ MVP 首选 |
| TUI 内 debug 面板 | 中高 | 未来可选 |
| stderr（仅非 TUI 模式） | 低 | 补充 print/server 模式 |

## 大致方向（待确认）

- `tracing-subscriber` + `tracing-appender`（non-blocking rolling file）
- 日志落点：`~/.xylitol/logs/xylitol.{yyyy-MM-dd}.log`
- 触发：`--debug` flag 或 `XYLITOL_DEBUG=1`
- 非 TUI 模式同步输出 stderr
- TUI 代码只调 `tracing::debug!()` 等宏（不依赖 infra）

## 待调研

- [ ] `tracing-appender` 是否需要独立 feature flag（避免不必依赖引入 print/server 模式）？
- [ ] `--debug` 是全局 flag 还是挂到 TUI？与现有 `CliArgs` 如何共存？
- [ ] 日志粒度设计：哪些关键路径需要打点（msg dispatch、stream 状态变换、render seam、mpsc 流控）？
- [ ] 是否需要 `RUST_LOG` 兼容？

## Capabilities（占位）

- `debug-log`（新增 spec）：观察管线

## Impact

- 低。纯新增依赖 + 初始化代码，不改变既有行为。
- 需要 design.md 确定最终方案后细化。
