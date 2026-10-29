---
depends_on: []
title: 会话能力覆盖盘：运行时即时开关工具与 MCP 服务器
needs_specs_change: true
branch: sdd/c2998-add-session-capability-overlay
base_branch: main
base_sha: 49d2170d3faf0de8915fda54dd7085fb3766221c
---

# 会话能力覆盖盘：运行时即时开关工具与 MCP 服务器

## Why

- #26 要求「运行时即时设置」：用户在会话进行中临时关掉某些工具或 MCP 服务器，而不必改全局配置或重启会话。
- 现行能力装配是静态的：工具由内核 `compose_tools` 一次性组装，MCP 服务器由 reload 全量重建；会话内没有任何「只影响本会话」的开关通道。
- 直接改全局配置会污染其他会话，且需要 reload（Exclusive）打断当前回合；用户需要的是**会话内临时覆盖**、下一回合边界生效、会话结束即丢。
- 端侧需要可发现、可撤销的交互面（TUI `/capabilities` 覆盖盘），并复用既有 status 区「下一回合生效」提示，避免静默生效。

## What Changes

- `xylitol-plugin-sdk` 新增能力 DTO：
  - `CapabilityKind::{McpServer, Tool}`。
  - `CapabilityEntry { id, kind, label, profile_enabled, session_disabled, effective_enabled, pending }`。
  - `CapabilitySnapshot { entries, pending }`。
  - 能力 ID 语义：工具 `tool:<name>`、MCP 服务器 `mcp:<server>`。
- `xylitol-runtime` 新增会话级覆盖层：
  - `XyDriver` 增加默认方法 `capability_snapshot` / `capability_pending` / `set_capability` / `clear_capabilities`（默认 unsupported，不破坏既有 driver）。
  - `XyInProcessDriver` 持有 base 工具集、MCP 服务器 id 列表、`session_disabled` 与 `applied` 集合。
  - 覆盖层只影响本会话的工具可见集：`tool:<name>` 直接过滤同名工具；`mcp:<server>` 过滤 `mcp__<server>__*` 工具。
  - 生效点固定为下一回合边界：`run` 开头应用 pending overlay，随后照常冻结工具表；当前回合不拆除已开工工具。
  - `McpReload` 改用 `install_base_tools`，使重载后的工具集仍受会话覆盖层约束。
  - 会话结束 / driver 销毁即丢，不写全局配置、不写 JSONL。
- `xylitol-protocol` 新增命令与方法表行：
  - `GetCapabilities {}` → `get_capabilities`（Readonly / Inline）。
  - `SetCapability { id, enabled }` → `set_capability`（Writer / Inline）。
  - `ClearCapabilities {}` → `clear_capabilities`（Writer / Inline）。
- `xylitol-kernel` 命令分派新增 `DispatchOutcome::Capabilities`，并序列化为快照 JSON。
- `xylitol-app-tui` 新增 `/capabilities`：
  - 打开替换 editor 的 `CapabilitiesSlot`（SelectList），汇总 enabled/total 与 pending 状态；Enter toggle、Esc 关槽。
  - toggle 经共享 dispatch 提交 `SetCapability`；成功后刷新槽与 status cue。
  - 当 `driver.capability_pending()` 为真，status 区显示 `Next turn: capability overlay` 待生效提示。
- 规格：
  - `agent-session` 增加「会话能力覆盖层」规则：会话内临时、下一回合生效、结束即丢。
  - `protocol-app` 增加能力方法表登记规则。
  - `app-tui-commands` 增加 `/capabilities` 面板与 pending 提示规则。

## Capabilities

- `agent-session`（会话能力覆盖层）
- `protocol-app`（能力命令方法表）
- `app-tui-commands`（`/capabilities` 覆盖盘）

## Impact

- **默认行为不变**：未调用覆盖命令时工具集与 MCP 装配与现状一致。
- **可回退**：`ClearCapabilities` 或会话结束即恢复；不触碰全局配置。
- **端侧新增入口**：TUI 多一个 slash 命令；Remote 端可用三个新方法做等价操作。
- **不做**：不持久化覆盖层；不做跨会话共享；不在本 change 引入通用「能力插件市场」。

## Non-Goals

- 不改变 ReAct 循环结构、工具批调度或权限裁决。
- 不把覆盖层写入 `config.yaml` / JSONL / session store。
- 不实现 MCP 服务器的动态启停（仅过滤其工具可见集）。
- 不为未交付的第三方插件市场增加能力注册中心。
