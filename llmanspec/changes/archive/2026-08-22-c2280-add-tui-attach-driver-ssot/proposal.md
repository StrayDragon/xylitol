---
depends_on: []
---

# CS 切分与传输调研

本票只交 `research/`，不实现。产品位在 c2300–c2305。

## Why

多窗 TUI 重复付 RSS / MCP / 启动税。现 Server 一把 `Mutex`、REST 未按 session 路由、TUI 从不 attach。需要归属与传输对照后再拆实现票。

## What Changes

1. `research/00`–`06`、`acp-interop.md`。
2. 不改 live specs，不接线 attach，不改运行时。

## 非目标

Ensure-running、Web 开闸、换编码、ACP 进核心（c2305）。
