---
depends_on:
  - c3035-add-ensure-host
title: 确保口上新二进制替换过期 Host
needs_specs_change: true
---

# 确保口上新二进制替换过期 Host

## Why

c3035 开箱确保本机 Host 后，`/healthz` 版本与当前客户端不符仍 fail-closed（提示 `kill <pid> && xylitol serve`）。用户升级二进制再开 TUI 时，旧 last-client / 显式 persistent 仍占 18790，必须手停才能进。本机确保口上「新来的替换旧来的」是同一套探活缝上的优化，不宜塞进 c3035 的 last-client idle 口径。

## What Changes（候选项，promote 时逐条重估）

- 本机确保口：healthz 声明 xylitol 且版本**旧于**当前客户端时，对该 pid 发 SIGTERM，等端口空闲，再 spawn `current_exe()`，等 `/healthz` 200 后 attach。
- 本机确保口：healthz 版本**新于**当前客户端时 fail-closed（升级客户端或 `xylitol serve stop`），不得杀掉更新的 Host，避免新旧客户端互杀。
- 显式非确保 `--attach`、`--no-ensure-host`：版本不符仍 fail-closed，不替换。
- 端口被非 xylitol 占用：不杀、不 spawn（沿用 r1787 / r3065）。
- attach 后协议版本不符仍按既有 fatal 终止，不重试风暴。

## 开放决策（propose 时拍板）

- 同 `CARGO_PKG_VERSION`、不同 git 构建：healthz 只有 version 字符串时无法区分；是否加构建身份。
- 显式 `xylitol serve`（persistent）被新 TUI 替换后，新进程走自动拉起的 last-client 还是继承 persistent。
- 替换窗口内第二客户端的 EADDRINUSE / 半关停竞态。

## Capabilities（拟）

- `cli-entry` r1391：确保口版本旧则替换后再 attach。
- `user-experience` r3065：替换失败与「Host 更新、客户端过旧」诊断分开。
- `server-core`：沿用 `serve stop` 的 SIGTERM 语义，不另造杀进程通道。

## Impact

只动本机确保路径；远程 attach 与 Print/嵌入不变。成功替换会断开仍连在旧进程上的客户端。

## 非目标

- 不把自动拉起缺省切到 persistent（C）；C 仍是 `serve.idle_timeout: false`。
- 不替换非回环、非确保口上的进程。
- 不靠应用层心跳轮询保活（存活仍是已认证 WS）。
- 不开 PR。

## 状态注记

> **deferred**：搁置在 `llmanspec/delayed-changes/`，未进活跃 graph。落实时经 `llman-sdd-propose` 迁回 `changes/`。
