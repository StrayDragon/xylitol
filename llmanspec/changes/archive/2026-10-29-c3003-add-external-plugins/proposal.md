---
depends_on: []
title: 第三方外进程插件最小切片：信任门禁 + JSON-RPC stdio 工具桥 + Bun demo
needs_specs_change: true
branch: sdd/c3003-add-external-plugins
base_branch: main
base_sha: fb6501e1a9ae8d26e4a8954eedb0942a2a9b5629
---

# 第三方外进程插件最小切片：信任门禁 + JSON-RPC stdio 工具桥 + Bun demo

## Why

- #29 要求第三方插件「可写可装」：官方 sdk 合同内贡献能力，核心不变量不可替换。
- 现行扩展入口只有官方插件 + 配置 + MCP / shell hook；内核已有 Chain 解析器与 admit 准入位，但没有可声明的第三方来源。
- 本 change 先交付**配置驱动的外进程插件最小切片**：信任门禁、newline-delimited JSON-RPC 2.0 stdio 握手、工具桥、失败隔离，并用 Bun/TS demo 验证。
- 不交付市场、签名、dylib 热载、同进程第三方 Rust 静态链入；这些另立 change。

## What Changes

- 配置 `external_plugins: Vec<ExternalPluginConfig>`：
  - `name` / `command` / `args` / `env` / `cwd` / `trusted` / `timeout_ms`。
  - 仅 `trusted: true` 条目装配；未 trusted、保留名、重复名、空 name/command 跳过并 warn。
- 内核 `external_plugin` 模块：
  - `ExternalClient`：spawn 子进程，newline-delimited JSON-RPC 2.0 over stdin/stdout。
  - `initialize` 握手：要求 `protocol_version: 1`、`tools` 能力、工具列表。
  - `ExternalPlugin`（rutis Plugin）：握手成功后把工具注册进 `HostRegistries.tools`，公开名 `{plugin}__{tool}`。
  - `ExternalTool`（XyTool）：转发 `tools/call`，结果文本回传；超时终止进程并报 `Timeout`。
  - 卸载 effect：移除工具并终止外部进程。
- 装配接线：
  - `boot_ctx` 从 ConfigService 读取 `external_plugins`，把受信条目注册为 Builtins 并追加 loader rows。
  - 复用既有 `Chain` + `Admit`；新增 `RESERVED_PLUGIN_NAMES` 防第三方覆盖核心 / 官方插件名。
- 配置产物与文档：schema / example / README（demo README）。
- Demo：`examples/plugins/bun-demo/plugin.ts`（echo / add），`scripts/lab_external_plugin_bun.py` 本地验证。
- 规格：新增 `infra-plugin` 三条规则与 BDD 场景。

## Capabilities

- `infra-plugin`（外进程插件信任、协议、工具桥、失败隔离）

## Impact

- **默认行为不变**：无 `external_plugins` 时零装配、零运行时成本。
- **安全边界**：外进程插件必须显式 `trusted: true`；核心 / 官方插件名不可被覆盖；进程失败只 warn，不阻断内核装配。
- **可回退**：移除配置即回到纯官方插件；工具公开名带插件前缀，便于审计与卸载。

## Non-Goals

- 不交付插件市场、清单注册中心、签名 / 分发。
- 不交付 dylib 热载、同进程第三方 Rust 静态链入。
- 不把每 token 流或 ReAct 内环交给第三方；插件只贡献工具。
- 不支持 git 地址解析、版本协商、权限能力声明（后续切片）。
- 不改变跨端信封或 rutis 监听器表。
