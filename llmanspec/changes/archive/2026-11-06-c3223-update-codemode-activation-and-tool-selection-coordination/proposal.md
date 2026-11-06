---
depends_on:
  - c3106-add-codemode-tool
title: 协调 codemode 插件激活与工具选择
branch: sdd/c3223-update-codemode-activation-and-tool-selection-coordination
base_branch: main
base_sha: 09488e2011402f59c55a2265092028766c204eec
---

## Why

`codemode.enabled` 决定是否装配插件、启动 Wasmtime、注册 `run_code`；`tools` 的 `+` / `-` 决定模型可见工具。两者不是冲突关系：启用 codemode 后，用户可以用 `-run_code` 隐藏该工具，这是合法选择。当前问题是插件仍会立即创建 runner，即使 `run_code` 未选择；以及 `+run_code` 但插件未启用时只得到 unknown tool 警告。

## What Changes

- 明确分层：`codemode.enabled` 是激活开关；`tools` 只做选择；`enabled + -run_code` 是合法组合，不报冲突。
- `RunCodeTool` 改为惰性 runner：首次实际调用才创建 `CodemodeRunner`；`-run_code` 或从未调用时零 Wasmtime engine。
- `+run_code` 但插件未启用：配置检查/启动日志 warning，明确“codemode disabled; run_code selection ignored”。
- 保持 `tools` 现有 `+` / `-` 语义；不改成 allowlist。
- 测试：disabled 零装配；enabled + `-run_code` 合法且无 engine；enabled + selected 首次调用才建 engine；`+run_code` 未启用 warning。

## Capabilities

- `agent-codemode`
- `runtime-config`
- `workspace-assembly`

## Impact

- kernel boot / config check：仅检查“选择但未激活”的 warning。
- codemode 插件：惰性 runner。
- runtime tool selection：不改变语义。
- 不改变 guest ABI、run_code schema、权限或隔离。

## Further Notes

c3218 保证声明基于选择后的 catalog；本 change 解决激活与选择的运行时成本。两者可并行，但都依赖 c3106 已归档。
