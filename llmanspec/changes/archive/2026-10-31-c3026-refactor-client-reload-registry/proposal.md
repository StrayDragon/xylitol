---
depends_on: []
title: 端侧重载参与者注册表
needs_specs_change: false
branch: sdd/c3026-refactor-client-reload-registry
base_branch: main
base_sha: 93d79cf90a90d4b790c7dd746d3257460f67ee8f
---

# 端侧重载参与者注册表

## Why

端侧 `/reload` 在 `run_interactive_reload` 内联 keybindings 与 themes，命令实现知道端侧机制细节；与宿主/插件侧注册表不一致。

## What Changes

- 端侧新增 `ClientReloadStep` 注册表：步骤含名称、阶段（`BeforeHost` / `AfterHost`）与执行闭包。
- `run_interactive_reload` 接收步骤列表，按阶段执行；宿主重载保持中间位置。
- 默认步骤（keybindings、themes）由 `default_client_reload_steps` 提供；host 循环与 harness 装配时传入。

## Capabilities

无产品行为规则变更：`app-tui-commands` r1191 已按「端侧本地机制」表述。

## Impact

- `crates/xylitol-app-tui`：`effects/reload.rs`、`effects/mod.rs`、`lib.rs`、`harness.rs`。
- 报告顺序不变：keybindings → 宿主步骤 → themes。

## 非目标

- 不改 `src/AGENTS.md`：端侧机制可由代码结构发现，避免架构文档按机制膨胀。
