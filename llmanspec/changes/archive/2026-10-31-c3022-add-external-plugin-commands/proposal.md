---
depends_on: []
title: 外部插件命令面（/plugin:<name>）
needs_specs_change: true
branch: sdd/c3022-add-external-plugin-commands
base_branch: main
base_sha: 4fa79099832cacf2b7e11c5ee808b50bf9c5db01
---

# 外部插件命令面（/plugin:<name>）

## Why

magpie 连接器需要 `/plugin:x-magpie refresh` 与状态查询；外部插件当前只能贡献工具，没有可调用的命令面。xylitol 的斜杠命令表是静态 SSOT，无法承载插件注册的命令。

## What Changes

- 外部插件 manifest 新增 `commands` 能力与命令声明。
- 内核把每个插件注册为 `plugin:<plugin-name>`，参数原样透传给插件。
- 协议新增 `Command::PluginCommand { plugin, args }`（方法名 `plugin_command`）。
- TUI 解析 `/plugin:*`、目录补全、busy 拒绝；结果以滚动提示呈现，不进模型上下文。
- 插件失败隔离：调用失败返回可观察错误，不崩溃。

## Capabilities

`infra-plugin`：新增 r3027（外部插件命令面）。
`app-tui-commands`：新增 r1206（插件斜杠命令）。
`protocol-app`：新增规则（PluginCommand 方法）。

## Impact

- `xylitol-plugin-sdk`：`XyPluginCommand` 与命令注册表。
- `xylitol-protocol` / `xylitol-runtime` / `xylitol-kernel` / `xylitol-app-tui`。
- 无配置面新增（沿用 `external_plugins`）。

## 非目标

- 不做插件命令的参数补全（v1 只列命令名）。
- 不做插件触发的模型热重载（另立 change）。
- 不做插件间命令转发。

## 不承诺

- 插件命令输出进入会话历史（明确不进）。
