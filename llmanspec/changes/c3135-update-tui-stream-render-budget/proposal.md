---
depends_on: []
title: TUI 流式渲染预算与合帧降频
needs_specs_change: false
---

# TUI 流式渲染预算与合帧降频

## Why

tmux 真机观测：1000 delta/s 时 debug TUI 在 5.54s 内消耗 5.53s CPU（约 100% 单核），release 在 1.97s 内消耗 1.66s（约 85%）；25–250 delta/s 只有约 1%。原因不是全量重绘本身，而是每个 token 触发一次全量 `UiModel` 同步 + scrollback 重建：runtime 先 yield `TextDelta` 再 yield `MessageUpdate`，TUI 合批只认 `TextDelta`/`ThinkingDelta`，`MessageUpdate` 把合批打断。渲染 lab 显示按 8 delta 合帧可把渲染时间降到 1/8.15。

## What Changes

- 把 `MessageUpdate` 纳入合批：30–50ms 窗口；每批只应用最新累积快照，丢弃中间重复；不改变消息语义。
- 去掉内容 delta 上的 `mark_ao_components_stale`；只有 dock / editor 高度变化才标脏，恢复 AO reproject 快路径与 16ms 节流。
- streaming tail 移出 upper cache，单独 generation；每帧只重生尾巴。
- busy tick 16ms → 33ms 或 50ms；spinner 按 c3130 定稿 100ms；内容帧由合批窗口驱动。
- thinking 尾加稳定前缀缓存；复用引擎 Markdown 实例缓存；流式期 `MermaidFenceMode::Off`，结束切 `Final`。
- `entry_fingerprint` 改增量 / 脏标记；避免每帧哈希完整 tool output / write_content。
- 避免每 delta 全量 clone `UiModel`；原地应用 / 借用。
- 可选：引擎 AO `set_lines` 增量 append/replace；本 change 先做应用侧，引擎 API 另开。

## Capabilities（拟）

无产品行为合约变更；只改渲染节奏与性能。`needs_specs_change: false`。若合批导致可观察的延迟语义变化（如状态条更新节拍），在 design 记录并由 harness 护栏，不改 spec。

## Impact

`crates/xylitol-app-tui/src/lib.rs`（合批）、`host/mod.rs`（stale / tick）、`layout/root/render.rs`（upper cache）、`widgets/scrollback/{mod,cache,live,paint}.rs`（tail / fingerprint / markdown）、`crates/xylitol-tui`（可选增量 API）。不改协议 / Host / 消息落盘。

## 非目标

- 不改 ReAct loop 的事件顺序；不改 `TextDelta` / `MessageUpdate` 语义。
- 不做引擎级结构化 StyledLine / 新渲染层。
- 不做终端写字节优化（tmux server CPU 已低于 TUI 一个数量级）。
- 不做全量 `set_lines` 重写；增量 API 后置。
- 不改流式 thinking 的可见性（c3120）。

## 状态注记

> **designed**：proposal + design 已完成；未写代码、未绑分支、未改 live specs。apply 时按 design 的「实施任务」生成 tasks.md。
