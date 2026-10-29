---
depends_on: []
title: LSP / DAP 工具插件：lspz 诊断符号 + dapz 调试语境
needs_specs_change: true
branch: sdd/c3004-add-lsp-dap-tools
base_branch: main
base_sha: c31ad674b9eef2791c2b38ae23ccc5427a2bf4d6
---

# LSP / DAP 工具插件：lspz 诊断符号 + dapz 调试语境

## Why

- #15 要求把 LSP 诊断 / 符号 / 悬停压进模型工具语境；#16 要求把 DAP 断点 / 栈 / 变量 / 求值压进语境。
- 现行扩展入口没有 LSP / DAP；手工读文件或让模型猜错误成本高。
- lspz / dapz 已提供 Agent SDK 与压缩输出；本 change 把它们包成官方工具插件，默认零成本。
- #16 原 delayed `c85-add-dap-layer` 只是空 trait 预留，已过时；本 change 直接交付可用 DAP 工具。

## What Changes

- 新官方插件 `xylitol-official-plugin-lsp`：
  - 配置 `LspPluginConfig`：`backend` / `language` / `workspace_root` / `compression`。
  - 工具：`lsp_diagnostics` / `lsp_symbols` / `lsp_hover`。
  - 首次工具调用懒启动 lspz `AgentHandle`；卸载时 shutdown。
  - 路径 / URI 解析：`path`（工作区相对或绝对）或 `uri`。
- 新官方插件 `xylitol-official-plugin-dap`：
  - 配置 `DapPluginConfig`：`backend` / `cwd` / `compression`。
  - 工具：`dap_launch` / `dap_stack` / `dap_scopes` / `dap_variables` / `dap_evaluate` / `dap_continue` / `dap_disconnect`。
  - 首次工具调用懒启动 dapz `AgentHandle`；`dap_disconnect` 释放会话；卸载时 disconnect + terminate。
- 配置 / 装配：
  - `AppConfig.lsp` / `AppConfig.dap` 为 `Option`；缺省零装配。
  - kernel `optional_plugin_rows` 在配置存在时追加 `lsp` / `dap` rows；`official_builtins` 注册两个插件。
- 配置产物与文档：schema / example。
- 规格：新增 `infra-lsp` 与 `infra-dap` 规则与 BDD 场景。
- 验证：basedpyright 诊断 roundtrip、debugpy launch / stack / evaluate roundtrip（ignored 单测，本地运行）。

## Capabilities

- `infra-lsp`（LSP 工具与零装配）
- `infra-dap`（DAP 工具与零装配）

## Impact

- **默认行为不变**：无 `lsp` / `dap` 配置时零装配、零运行时成本。
- **可回退**：移除配置即回到无 LSP/DAP 工具；工具名稳定，便于配置 `tools` 选择。
- **失败可观察**：backend 启动 / 查询失败经 `XyToolError` 回传，不静默成功。
- **依赖**：新增 `lspz` / `dapz`（agent-sdk）依赖；只在配置启用时 spawn 子进程。

## Non-Goals

- 不做 LSP / DAP 全协议覆盖；只交付诊断 / 符号 / 悬停与调试核心操作。
- 不把每 token 流或 ReAct 内环交给 LSP / DAP。
- 不做 LSP 自动补全 / 重构 / 格式化工具（后续可加）。
- 不做 DAP daemon 复用 / 多会话池；每次插件实例懒启动一个 AgentHandle。
- 不为未配置的 LSP / DAP 付运行时成本。
