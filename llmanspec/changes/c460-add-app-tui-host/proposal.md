---
change_id: c460-add-app-tui-host
title: "app-tui-host：host 循环、终端生命周期、即时 debug log"
status: purpose-draft
priority: 460
depends_on: ["c450-revise-app-tui-contract", "c455-add-package-tui-input-listener"]
author: agent
track: B
---

# c460-add-app-tui-host

> **status: purpose-draft**

## Why

`src/app/tui/run` 仍为占位；需要 host 驱动引擎 + 异步合流 + 可靠终端 restore。

## Purpose

实现可进入的空壳 TUI：`tokio::select!` 合流输入/事件/取消；panic/信号 restore；debug 构建默认写即时 log（可 `tail -f`），release 默认关。

## What Changes（意向）

1. `tui` 进 default features（与 c450 合约一致）。
2. host：`dispatch_event` / `request_render` / `try_render` / `idle_tick`；禁止产品路径 `TUI::start()`。
3. 终端：raw mode、Drop restore、panic hook、SIGTERM/SIGHUP 尽力 restore。
4. Resize：失败时**干净退出进程并 restore**，禁止卡死模拟器。
5. 最小尺寸：宽&lt;40 或高&lt;6 → 全屏一行提示「请放大终端」，不 panic；恢复尺寸后继续。
6. 日志：debug profile 默认 `~/.xylitol/logs/xylitol.log`（或既有路径）；`AGENTS.md` 写明 `tail -f`；release 默认关，可用 `XYLITOL_DEBUG`/`RUST_LOG` 打开。
7. 空壳布局：transcript 空 + editor + footer 占位。

## Capabilities

- `app-tui-host`

## Out of scope

- XyEvent 渲染、slash 语义、steer
