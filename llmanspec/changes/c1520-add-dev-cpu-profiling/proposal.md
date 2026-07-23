---
change_id: c1520-add-dev-cpu-profiling
title: Dev CPU profiling（火焰图 / samply / pprof）
status: purpose-draft
priority: 1520
depends_on: []
author: agent
---

# c1520-add-dev-cpu-profiling

## Why

TUI 长会话性能优化（c1500 / c1510 / 后续 c1505）目前仍大量依赖**体感**与 harness 计数。需要可复现的 CPU 热点图，避免「感觉卡」与真实热点错位。

## 意向（延后正式 propose）

1. **`[profile.profiling]`**：基于 release、保留 debug 符号、不 strip
2. **可选 feature `cpu-profile`**：`pprof` 采样，退出或信号写出 `target/profile/*.svg` / pb
3. **`just profile-*`**：文档化 `samply record` / `cargo flamegraph`（零强制 deps）与可选 pprof 路径
4. （可选）`criterion` bench：`render_scrollback` / streaming assistant paint

## Out of scope（本 draft）

- 改生产默认二进制行为
- 以 OS CPU% 作 CI 硬闸
- 实现 c1505 viewport slice

## 提案顺序（人类约定）

1. **本 change（c1520）** — 延后 promote
2. **c1505** viewport slice
3. 其他

## Status

**purpose-draft** — 表头 Markdown 风格优先落地后再回来 promote。

## Ethics

- risk_level: low
- prohibited_actions: 默认开启采样拖慢日常路径；把 flamegraph 当作 MUST 产品行为
