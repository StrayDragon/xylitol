---
depends_on:
  - c3106-add-codemode-tool
title: 协调 codemode 插件激活与工具选择
---

## Why

当前链路把两件事分开：

- `codemode.enabled` 决定是否装配 codemode 插件、启动 Wasmtime、注册 `run_code`。
- `tools` 的 `+` / `-` 在 driver 装配后选择模型可见工具。

结果是：`codemode.enabled=true` 但 `tools: [-run_code]` 仍会启动 Wasmtime；`tools: [+run_code]` 但未启用插件时只得到 unknown tool 警告。插件范围与工具选择没有形成一致的配置语义。

## What Changes

- 明确激活与选择分层：`codemode.enabled` 是唯一激活开关；`tools` 只做选择，不得隐式启动插件。
- `+run_code` 但插件未启用：配置校验/启动日志给出明确 warning，不静默忽略。
- `codemode.enabled=true` 但 `run_code` 被 `-run_code` 禁用：给出 warning；插件仍可注册，但 runner 延迟到首次实际调用才创建，避免无谓启动 Wasmtime。
- `RunCodeTool` 改为惰性 runner：首次 `execute` 才创建 `CodemodeRunner`；未选择/未调用时零 engine。
- 保持 `tools` 现有 `+` / `-` 语义；不改成 allowlist。
- 测试：disabled 零装配；enabled+disabled 无 engine；enabled+selected 首次调用才建 engine；`+run_code` 未启用 warning；选择语义不变。

## Capabilities

- `agent-codemode`
- `runtime-config`
- `workspace-assembly`

## Impact

- kernel boot / config validation：跨 section 一致性 warning。
- codemode 插件：惰性 runner。
- runtime tool selection：不改变语义，只消费结果。
- 不改变 guest ABI、run_code schema、权限或隔离。

## Further Notes

c3218 只保证声明基于选择后的 catalog；本 change 解决激活与选择的配置联动。两者可并行，但都依赖 c3106 已归档。
