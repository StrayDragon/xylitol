---
change_id: c1100-update-runtime-context-hot-reload
title: "Context 文件热重载：AGENTS.md / SYSTEM 等不改历史"
status: draft
priority: 1100
apply_band: P2-system
depends_on: []
author: agent
track: R
wave: reload-foundations
domain: runtime
ethics:
  risk_level: medium
  prohibited_actions:
    - 重载改写历史 user/assistant/toolResult
    - 未信任时加载项目 context / 项目 SYSTEM / APPEND
  required_evidence:
    - 重载前后 session 文件历史条目不变
    - 新一轮 system 反映新 AGENTS/SYSTEM/append
  escalation_policy: 若须「中途插入 env 消息」需显式产品确认
---

# c1100-update-runtime-context-hot-reload

## Why

启动时已加载 context（AGENTS.md / CLAUDE.md）与 SYSTEM/APPEND。`/reload`（c1120）需要对齐 pi：重读 context，**影响后续轮次**，不改写已持久化历史。

`DefaultResourceLoader` 文档声称 `reload()`，但实现缺失；agent 仅有 `set_system_prompt`，无法替换 `context_files` / append。

## Purpose

1. 在 `runtime_protocol` 引入窄 **`XyReloadable`**（关联 `Outcome`，编译期约束；**不是** `dyn` 插件注册表）。
2. `DefaultResourceLoader` 实现真正的磁盘重扫 + `XyReloadable`。
3. Agent / Driver seam：应用 context 快照并 `rebuild_system_prompt`，**MUST NOT** mutate session 历史。
4. `app/core` 提供与 bootstrap 同 Trust 语义的 `reload_prompt_context` 助手（供 host / 未来 `/reload`）。

## What Changes

- `runtime_protocol`：`XyReloadable` +（可选）`ResourceReloadOutcome`
- `infra/resource`：`DefaultResourceLoader::reload` 清缓存再 `load_all`
- `agent`：`apply_prompt_resources`（或等价）更新 context/system/append 并重建
- `app/core`：`InProcessDriver` 转发 + `reload_prompt_context`（Trust 闸与 bootstrap 一致）
- delta：`runtime-resource-discovery` · `agent-prompt` · `agent-runtime`

## Capabilities

- `runtime-resource-discovery`（modify）
- `agent-prompt`（modify）
- `agent-runtime`（modify）

## Out of scope

- `/reload` 总控 UI（c1120）
- themes 解析 / `LayoutTheme`（c1095）
- skills 运行时产品化（c1085）
- keybindings（c1090 已归档）
- `dyn XyReloadable` 注册表 / 扩展市场

## Impact

- 解锁 c1120 的 context 半边；与 c1095 独立可并行
- arch_guard：TUI 仍只经 `app::core`，不 reach `infra::resource`
