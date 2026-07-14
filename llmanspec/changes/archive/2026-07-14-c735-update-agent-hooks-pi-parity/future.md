---
change_id: c735-update-agent-hooks-pi-parity
---

# Deferred → c995 域 draft

本 change 已接线之外的项，已拆为 **purpose-draft**（`domain: c995`）。**c735 已归档**；各草案 `depends_on: c735` 已满足，升格 full 后可 apply。

| 草案 | 覆盖 |
|------|------|
| [`c995-update-agent-hooks-driver-session`](../c995-update-agent-hooks-driver-session/proposal.md) | Driver：`session_before_tree` / `session_tree` / `session_before_switch` / `session_shutdown` |
| [`c996-update-agent-hooks-model-select`](../c996-update-agent-hooks-model-select/proposal.md) | `model_select` / `thinking_level_select` |
| [`c997-update-agent-hooks-bang-input`](../c997-update-agent-hooks-bang-input/proposal.md) | `user_bash` / `input`（TUI bang·提交） |
| [`c998-update-infra-completions-provider-hooks`](../c998-update-infra-completions-provider-hooks/proposal.md) | Completions 完整 HTTP 三缝 |

## 仍留 later（未单独立 draft）

| 项 | 说明 |
|----|------|
| `project_trust` 脚本参与 | trust 子系统已独立；需要时另开 |
| pi `ctx.ui` / custom tool render | UI 扩展面，另开大 change |

## drop

| 项 | 原因 |
|----|------|
| 成功 SSE body 默认进 hook | 与 pi 一致；抓包装 [`c999`](../c999-add-infra-provider-traffic-capture/proposal.md) |

## done in c735

- Provider 三缝（Responses/Anthropic）+ `XyHookBus`
- ReAct：tool/context + agent/turn/message + `agent_settled`
- Session：start、before_compact/compact、`before_fork`
- BDD provider 非空；main spec h1/h7 更名
