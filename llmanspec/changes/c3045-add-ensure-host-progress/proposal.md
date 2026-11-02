---
depends_on:
  - c3035-add-ensure-host
title: 确保 Host 等待过程可见
needs_specs_change: true
---

# 确保 Host 等待过程可见

## Why

c3035 开箱会 ensure 本机 Host：未在听则 spawn `current_exe() serve --lifecycle last-client`，轮询 `/healthz` 直到 200 再 attach。这段等待在 TTY 上**没有进度**：首次装配（插件 / 配置 / otel）可达数百毫秒到数秒，终端像卡住。OpenCode 在拉起共享服务时有「start server」TUI 动画，把同一段等待做成可感知过程。

本草案只美化**确保等待**；last-client idle、凭据发现、新二进制替换旧 Host（c3040）不在范围。

## What Changes（候选项，promote 时逐条重估）

- 产品 TUI 在本机确保口**需要等待** Host 就绪时（spawn 后未立即 200，或 Starting / 凭据尚未写出），TTY 必须呈现短进度：spinner + 固定短词（拟 `Starting host`，与 status 槽 `Reloading` / `Assembling` 同形：accent spinner + 贴左短词）。
- 本机 Host **已经 Ready**：不得闪过等待面，直接 attach 进产品画布。
- 等待失败：收起进度，落到既有 r3065 可操作诊断（stderr），不得只剩空白或 mux 垃圾。
- 进度面在**进入产品 TUI 画布之前**（ensure / attach 握手未完成不得 `TerminalGuard::enter` 产品 raw/alt-screen）。非 TTY、`--no-ensure-host`、print / 嵌入：无动画。
- 文案与色票宜复用产品 Palette / Loader，不得另做第二套 theme。

## 开放决策（propose 时拍板）

- 形态：产品画布前的一行 TTY spinner，还是短寿命 boot 场景（仍须在 Host Ready 后才进产品 host 循环）。
- 是否暴露装配相位（listening / assembling / ready），还是全程一句 `Starting host`。
- 已有 Host 但 WS 握手较慢：是否也走同一进度，还是只覆盖 spawn→healthz。

## Capabilities（拟）

- `cli-entry` r1391：确保等待在 TTY 上可观察。
- `user-experience` r3065：等待中 vs 失败诊断分开；失败仍可操作。
- `app-tui-fixed-zone`：短词与 Loader 形制对齐（若不进产品 status 槽则只对齐视觉，不改 idle status 0 行）。

## Impact

只动产品 TUI 确保入口的等待呈现；Host 生命周期与 Print 不变。成功路径多一段可取消或不可取消的短动画（取消是否映射进程退出，propose 时定）。

## 非目标

- 不改 last-client 1→0 idle、不改凭据 / `serve.json` 时序。
- 不实现 c3040 新替换旧。
- 不把等待做进产品 session 画布的假 Busy / 假 Reloading。
- 不给 print 做启动动画。
- 不开 PR。
