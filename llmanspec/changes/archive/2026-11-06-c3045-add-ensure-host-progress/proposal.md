---
depends_on:
  - c3035-add-ensure-host
title: 确保 Host 等待过程可见
needs_specs_change: true
branch: sdd/c3045-add-ensure-host-progress
base_branch: main
base_sha: fd2c29a51fa6d8dde04cfbb102724a989f6a242d
---

# 确保 Host 等待过程可见

> **planned**：design / tasks 已写；apply 前需 attach + specs。

## Why

c3035 开箱会 ensure 本机 Host：未在听则 spawn `current_exe() serve --lifecycle last-client`，轮询 `/healthz` 直到 200 再 attach。这段等待在 TTY 上**没有进度**：首次装配（插件 / 配置 / otel）可达数百毫秒到数秒，终端像卡住。OpenCode 在拉起共享服务时有「start server」TUI 动画，把同一段等待做成可感知过程。

本草案只美化**确保等待**；last-client idle、凭据发现、新二进制替换旧 Host（c3040）不在范围。

## What Changes

- 产品 TUI 在本机确保口**需要等待** Host 就绪时，TTY 呈现一行短进度：spinner + 固定短词 `Starting host`，与 status 槽 `Reloading` / `Assembling` 同形。
- 本机 Host **已经 Ready**：不得闪过等待面，直接 attach 进产品画布。
- 等待失败：收起进度，落到既有 r3065 可操作诊断（stderr），不得只剩空白或 mux 垃圾。
- 进度面在**进入产品 TUI 画布之前**（ensure / attach 握手未完成不得 `TerminalGuard::enter` 产品 raw/alt-screen）。非 TTY、`--no-ensure-host`、print / 嵌入：无动画。
- 文案与色票宜复用产品 Palette / Loader，不得另做第二套 theme。

## 深挖结论（2026-11-04）

- 形态：产品画布前**一行 TTY spinner**；不是 boot 场景，不画 header / 假 session。
- 文案：固定短词 `Starting host`；与 status 槽 `Reloading` / `Assembling` 同形（accent spinner + 贴左短词）。
- 相位：MVP 只一句 `Starting host`；listening / assembling / ready 相位后置。
- 延迟：150ms 启动；80ms 帧；ensure 在 150ms 内完成则不输出，避免闪。
- 取消：MVP 不提供自定义取消；等待期间 Ctrl+C 由终端 SIGINT 终止。
- 覆盖：只覆盖 spawn→`/healthz` ready；已有 Host 但 WS 握手慢不在本 change。
- 非 TTY / `--no-ensure-host` / 显式非确保 `--attach` / print / 嵌入 / `server` feature 关闭：无动画。
- 失败：清行后落既有可操作诊断；不得只剩空白。
- 顺序：进度在 trust gate 之前结束；`TerminalGuard::enter` 不得提前。
- 设计 / 任务：[design.md](./design.md) / [tasks.md](./tasks.md)。

## Capabilities（拟）

- `cli-entry` r1391：确保等待在 TTY 上可观察。
- `user-experience` r3065：等待中 vs 失败诊断分开；失败仍可操作。
- 不新增 `app-tui-fixed-zone` 条款：等待面在产品画布前，不是固定区；视觉形制对齐产品 Loader 即可。

## Impact

只动产品 TUI 确保入口的等待呈现；Host 生命周期与 Print 不变。成功路径多一段不可自定义取消的短动画；等待期间 Ctrl+C 由终端 SIGINT 终止。

## 非目标

- 不改 last-client 1→0 idle、不改凭据 / `serve.json` 时序。
- 不实现 c3040 新替换旧。
- 不把等待做进产品 session 画布的假 Busy / 假 Reloading。
- 不给 print 做启动动画。
- 不开 PR。
