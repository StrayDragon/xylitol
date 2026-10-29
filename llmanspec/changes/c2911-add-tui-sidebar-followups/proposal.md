---
depends_on: []
title: 会话侧栏与覆盖弹出框的命中、宽度与三态交互
needs_specs_change: true
---

# 会话侧栏与覆盖弹出框的命中、宽度与三态交互

## Why

- c2875 / c2905 / c2910 已落地覆盖弹出框与会话侧栏，但留下两类已知限制：
  - 侧栏带与覆盖弹出框没有命中注册：点按不关闭，鼠标选区会穿透到内容区，滚轮也会越界滚动内容区。
  - 侧栏宽度固定 24 列，且缺少“关 → 覆盖弹出框 → 侧栏”的单键循环。
- 侧栏当前是实验占位内容；把交互机制先补齐，后续业务字段回归时不再重复改输入/命中路径。

## What Changes

- 扩展 `HitSpace`：新增 `Chrome` 命中空间，用于侧栏、覆盖弹出框与分隔条。
- 注册 chrome 命中区：点击侧栏带收起侧栏；点击覆盖弹出框收起弹出框；点击分隔条循环侧栏宽度；该带内按下/拖拽不启动选区。
- 侧栏宽度改为可调档位（24 / 32 / 40），保证内容区至少 40 列；新增键位 `app.session.sidebarWidth`（默认 `alt+shift+u`）循环档位。
- 新增键位 `app.session.detailsCycle`（默认 `alt+o`）：关 → 覆盖弹出框 → 侧栏 → 关；窄档侧栏不可用时降级为弹出框。
- 侧栏带内滚轮只作用于侧栏自身（当前占位内容不溢出时保持 0），不得滚动内容区。
- 更新 `app-tui-fixed-zone` 的 `r2903` / `r2904` 与场景。

## Capabilities

- `app-tui-fixed-zone`：会话详情覆盖弹出框与会话侧栏的命中、宽度与三态交互。

## Impact

- `crates/xylitol-tui/src/pointer.rs`：`HitSpace::Chrome` 与捕获区点击派发。
- `crates/xylitol-app-tui/src/layout/root/`：宽度档、三态循环、chrome 命中区、侧栏滚动状态。
- `crates/xylitol-app-tui/src/host/mod.rs` / `lib.rs`：命中同步、点击派发、滚轮分流。
- `crates/xylitol-app-tui/src/keybindings.rs`：新增两个键位 id。
- `llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature`：规则与场景。
- `tests/bdd/steps_app_tui_host.rs` + `bindings_app_tui_host.rs`：BDD 场景。

## Out of Scope

- 不实现侧栏业务字段（仍待 c2912 正式业务代码定义）。
- 不实现拖拽分隔条（本轮用键位与点击分隔条切换档位）。
- 不持久化侧栏宽度到配置。
- 不改覆盖弹出框的字段集合与 z 序。
