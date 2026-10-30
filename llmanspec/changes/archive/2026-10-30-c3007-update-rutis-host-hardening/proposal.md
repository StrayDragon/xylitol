---
depends_on: []
title: rutis host 硬化：错误出口、启动诊断与后续顺序
needs_specs_change: true
branch: sdd/c3007-update-rutis-host-hardening
base_branch: main
base_sha: 029b3d9aa36ed7791fa9916880f7a2f7a966c90d
---

# rutis host 硬化：错误出口、启动诊断与后续顺序

## Why

- rutis 0.19 的默认 `ErrorSink` 把异步错误直写 stderr；这违反 `infra-observability` 的 file-only 观测约束，TUI Inline / DSR 有被污染的风险。
- `wait_active` 超时只列 `name:state`，没有说明“还缺哪个服务”，排障成本高；0.19 的 `Loader::waits` 已能给出缺失服务与原因。
- rutis 0.19 的 host 侧还有若干更推荐的写法（`PluginFactory` / `plugin_with`、`Key<T>`、`shutdown_with_timeout`、`report_waiting_after` 等）。本 change 只处理当前有明确收益的部分，其余按 ROI 排后续顺序，避免一次性大改。

## What Changes

- **错误出口**：`Kernel::boot` 改用 `Ctx::root_with_sink` 安装自定义 `ErrorSink`，把 rutis 异步错误经 `log` 写入既有 file-only 观测管线；不再直写 stderr。
- **启动诊断**：`wait_active` 在自检超时时用 `Loader::waits` 列出每个等待中的 row 缺哪些服务、为什么缺；无 waits 时回落到现有 state 列表。
- **风格收口**：`FiberView` 直接 `await`（去掉无意义的 `(&view).await`）；`boot_ctx` 用 `root_with_sink` 后不再依赖 `Ctx::root()` 的默认 sink。
- **规格**：
  - `infra-observability`：补 rutis 异步错误出口规则（由 kernel 单测守护；BDD 需全局 logger，暂不转写）。
  - `workspace-assembly`：补启动自检超时诊断规则与场景。

## 非目标 / 后续顺序

本 change 不处理以下项，按 ROI 留给后续 change：

1. `shutdown_with_timeout` 优雅关闭（host / serve / TUI 生命周期）——需要单独设计 `Kernel` 所有权与 `serve` 注册文件收口。
2. `PluginFactory` + `ctx.plugin_with`——核心插件目前不需要 live config update；等真实需求再上。
3. `Key<T>` / `provide_at` / `get_at`——当前服务按类型区分，没有同类型多 provider。
4. `report_waiting_after` / `name_keys_with`——诊断增强，可与后续 host 诊断 change 合并。
5. `rutis_bridge::thrown` / `rutis::config::check`——当前没有跨进程 rutis runtime，app config 也不走 rutis row config。

## Capabilities

- `infra-observability`（rutis 异步错误出口）
- `workspace-assembly`（启动自检超时诊断）

## Impact

- **行为变化**：rutis 异步错误从 stderr 转到 `log` 文件；boot 超时错误文案增加缺失服务信息。
- **不变**：插件写法、loader rows、事件/服务模型、产品信封、TUI 渲染主路径不变。
- **可回退**：恢复默认 `Ctx::root()` 与旧 state-only 文案即可。

## Further Notes

后续顺序（建议）：

1. `shutdown_with_timeout` 生命周期 change（最高 ROI，但改动面最大）。
2. host 诊断增强（`report_waiting_after` / `name_keys_with`）。
3. `PluginFactory` / `plugin_with`（仅在有 live config update 需求时）。
4. `Key<T>`（仅在出现同类型多 provider 时）。
