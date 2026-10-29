---
depends_on: []
title: Agent 预设：persona / system prompt / 工具目录按会话选择
needs_specs_change: true
branch: sdd/c3002-update-agent-presets
base_branch: main
base_sha: 4e31dd076599ff6588cb2bd03155804601a8188e
---

# Agent 预设：persona / system prompt / 工具目录按会话选择

## Why

- #37 要求「按会话选择 agent 工作方式」：预设声明 persona、system prompt 段与工具目录，而不是只靠全局配置与内置 prompt。
- 现有 `agents.profiles` 已支持 `model` + `system_prompt`，但没有 persona、工具目录，也不能按会话显式选择预设（只能改 `agents.default_profile`）。
- 本 change 先交付**静态预设目录 + 按会话选择**的最小切片；不 fork ReAct、不做 OO-Harness 派生/router、不做预设工坊 UI。

## What Changes

- `AgentProfile` 扩展：
  - `persona: Option<String>`：人设短句。
  - `tools: Vec<String>`：该预设的模型可见工具选择列表（`+name` / `-name` / 裸名）。
- `resolve_profile`：
  - persona 与 system_prompt 均非空时，按 `persona → 空行 → system_prompt` 拼接为系统提示。
  - 返回 `tools`；profile tools 非空时覆盖全局 `tools` 列表，为空时沿用全局。
- CLI / bootstrap：
  - `TuiSurfaceArgs` / `PrintSurfaceArgs` 增加 `--agent-profile <name>`。
  - `BootstrapInput` 增加 `agent_profile`；显式预设不存在时硬失败（`AgentProfileNotFound`），不得静默回退默认预设。
  - `ResolvedAssembly.tool_features.tool_selection` 采用 profile tools（非空时）。
- 配置 / 文档：
  - `configs/config.schema.json` 由生成器更新；`configs/example.yaml` 增加 `agents.profiles` 注释样例。
  - README 增加 `--agent-profile` 与 persona/tools 说明。
- 规格：
  - `runtime-config` 新增 `agent-profile-preset-fields` 规则与两个场景（persona+工具目录、未知预设硬失败）。

## Capabilities

- `runtime-config`（agent profile 字段与按会话选择）

## Impact

- **默认行为不变**：未使用 `--agent-profile` 且未配置 profile 时，沿用现有默认 profile / 全局 tools。
- **可回退**：移除 profile 字段或改回 `agents.default_profile` 即回到旧行为。
- **显式失败**：未知 `--agent-profile` 硬失败，避免用户误以为预设生效。

## Non-Goals

- 不实现 OO-Harness 的继承 / 派生 / router。
- 不实现预设工坊可视化或端侧预览面板。
- 不做极简模式（首轮 minimal → 首次工具调用后扩展）。
- 不引入插件提供的预设元数据注册中心（后续 #29/#35 方向）。
- 不改 ReAct 内环、会话历史或 cache 前缀策略。
