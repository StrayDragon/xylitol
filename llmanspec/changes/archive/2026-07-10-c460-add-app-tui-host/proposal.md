---
change_id: c460-add-app-tui-host
title: "app-tui-host：host 循环、终端生命周期、即时 debug log"
status: full
priority: 460
depends_on: ["c450-revise-app-tui-contract", "c455-add-package-tui-input-listener"]
author: agent
track: B
---

# c460-add-app-tui-host

## Why

`src/app/tui/run` 仍为占位；需要 host 驱动引擎 + 异步合流 + 可靠终端 restore。

## Purpose

实现可进入的空壳 TUI：host 合流输入/resize/idle/取消；panic/信号 restore；debug 构建默认写即时 log；最小尺寸友好提示。

## What Changes

1. `tui` 已在 default features（保持）。
2. host：`dispatch_event` / `request_render` / `try_render` / `idle_tick`；产品路径禁止 `TUI::start()`。
3. 终端：raw mode、Drop restore、panic hook、SIGTERM/SIGHUP 尽力 restore。
4. Resize：触发重绘；极端失败干净退出并 restore。
5. 最小尺寸：宽&lt;40 或高&lt;6 → 全屏一行「请放大终端」；恢复后继续。
6. 日志：debug profile 默认 `~/.xylitol/logs/xylitol.log`；AGENTS 写明 `tail -f`。
7. 空壳布局：transcript 占位 + bordered editor + footer。
8. **Harness**：Virtual/Test terminal + 可注入 `HostEvent` 覆盖 min-size、dispatch、无 `start()`。

## Capabilities

- `app-tui-host`

## Out of scope

- XyEvent 渲染（c465）、slash 语义（c480）、steer 键接线（c480）
