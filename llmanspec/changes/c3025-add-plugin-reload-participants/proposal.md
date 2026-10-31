---
depends_on: []
title: 插件注册具名重载参与者
needs_specs_change: false
branch: sdd/c3025-add-plugin-reload-participants
base_branch: main
base_sha: 3f347a77517c546028ff0990b05811d4667cd4ed
---

# 插件注册具名重载参与者

## Why

`/reload` 的宿主步骤已由注册表驱动，但插件只能经 `SessionPrepareReason::Reload` 事件参与，报告里没有插件自己的具名步骤。MCP 的 `mcp` 步骤仍由 kernel 内置占位参与者产出。

## What Changes

- `xylitol-plugin-sdk` 新增 `ReloadEvent` / `ReloadParticipant` / `ReloadHub`（插件侧注册表）。
- kernel 核心插件 `registries` 提供 `ReloadHub` Ctx 服务。
- 宿主重载运行器在宿主内置参与者之后遍历 `ReloadHub` 参与者，追加具名步骤。
- mcp 插件在装配时注册 `mcp` 参与者；kernel 的内置占位 `mcp` 参与者移除。
- `src/AGENTS.md` 增补「重载参与者」架构约束。

## Capabilities

无产品行为规则变更：`app-tui-commands` r1191 已按「已注册的宿主热重载机制」表述。

## Impact

- `xylitol-plugin-sdk`、`xylitol-runtime`（ctx getter）、`xylitol-kernel`（registries / 运行器）、`xylitol-official-plugin-mcp`。
- 步骤顺序：宿主内置（skills / context / models）→ 插件（mcp）。

## 非目标

- 端侧 keybindings / themes 仍属客户端本地机制（不接 `ReloadHub`）。
