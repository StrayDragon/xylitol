---
depends_on: []
title: 模型热重载：/reload 重建模型注册表
needs_specs_change: true
branch: sdd/c3023-add-model-reload
base_branch: main
base_sha: 8af99e1b5ae3be49d386b025b23b193948875716
---

# 模型热重载：/reload 重建模型注册表

## Why

magpie 连接器刷新后写入新的模型文件；当前 `/reload` 只重载 skills / context / themes / MCP，模型注册表在 boot 时构建，新增模型必须重启 xylitol 才可见。`/plugin:x-magpie refresh` 因此要求用户重启，刷新体验不完整。

## What Changes

- `/reload` 增加 `models` 步骤：重新读取配置层，重建模型注册表并应用到运行中的 agent。
- 当前模型在新注册表中仍存在时保留；失效时回落到默认模型，无默认则首个可用模型；无模型时保留旧注册表并给出可观察诊断。
- 当前 thinking level 在新模型支持时保留，不支持时回落。
- TUI 重载后刷新模型目录（`/model` 补全与模型列表）。

## Capabilities

`runtime-model-registry`：新增规则（重载重建注册表）。

## Impact

- `xylitol-runtime`：`ModelManager::replace_registry`、`AgentRuntime::replace_model_registry`。
- `xylitol-kernel`：`build_model_registry` 可见性、reload hook 新增 `models` 步骤。
- `xylitol-app-tui`：重载后刷新模型目录。

## 非目标

- 不做配置层监听（文件变更自动重载）。
- 不做运行中 provider 热替换（仍按注册表重建）。
- 不改 boot 时的模型装配逻辑。

## 不承诺

- 重载期间进行中的回合（reload 已有互斥与取消语义，不在本 change 扩展）。
