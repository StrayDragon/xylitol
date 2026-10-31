---
depends_on: []
title: 重载参与者注册表：/reload 不再硬编码机制
needs_specs_change: true
branch: sdd/c3024-refactor-reload-registry
base_branch: main
base_sha: 99fb816cc0ad8084205c9c2ca61fae3e8bbad640
---

# 重载参与者注册表：/reload 不再硬编码机制

## Why

`/reload` 的宿主实现（`ProductReload`）把 skills / MCP / context / models 四个步骤写死在命令实现里。通用命令因此知道各机制的内部细节；新增机制必须改命令实现，无法由机制自行注册。

## What Changes

- 新增宿主重载参与者注册表：机制实现 `ReloadParticipant` 并注册；`/reload` 只遍历注册表、收集步骤报告。
- 内置机制（skills / MCP / context / models）改为注册参与者；行为、步骤名与顺序保持不变。
- 插件侧沿用既有 `SessionPrepareReason::Reload` 事件订阅（观察者）参与重载。
- 更新 `app-tui-commands` 的 reload 规则：宿主机制集合由注册表决定，不在规则中固定列举。

## Capabilities

`app-tui-commands`：修改 reload 规则陈述。

## Impact

- `xylitol-runtime`：`ReloadRegistry` / `ReloadParticipant` / `ReloadContext`；驱动持有注册表。
- `xylitol-kernel`：内置参与者与注册；`ProductReload` 改为遍历器。
- 无产品行为变化（步骤名、顺序、失败语义不变）。

## 非目标

- 不改端侧 keybindings / themes 的本地重载（客户端自有能力）。
- 不新增插件注册 API（插件经既有 session-prepare 事件参与）。

## 不承诺

- 注册表不保证跨进程插件（外进程）参与宿主重载。
