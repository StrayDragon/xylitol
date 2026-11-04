---
depends_on: []
title: "Host workspace set binding from client"
---

## Why

Host 当前启动即绑定 cwd；`new_session` 用 host 进程 cwd；多 client / 多 workspace 无法隔离。MCP per-workspace 与 trust 需要 workspace 从 client 传入。一个 session 可以绑定一个 workspace，也可能绑定多个受信任 workspace（1:N，非无限；多数 1:1）。

## What Changes

- host boot 不再要求 workspace；workspace set 可选。
- client 接入时传 workspace set：primary workspace + additional trusted workspaces。
- 推荐在 `subscribe` 中正式化 `workspaces` 字段（list）；现有 `cwd` 作为 primary 的兼容入口。
- primary workspace = TUI/其他客户端启动目录；resume 时 = session 存储的 primary。
- additional workspace 只能由人类添加/移除：TUI slash 命令或 host unary；不得暴露为 LLM 可调用工具。
- workspace set 的 mid-session 变更只允许人类操作；LLM 不得自行 add / remove / switch。
- trust 按 workspace set 逐个解析；新加入的 workspace 走 trust 门禁。
- host 按 session 绑定 workspace set；`new_session` 用 primary workspace，不用 host cwd。
- materialize driver、MCP pool、tool catalog 使用 session 的 workspace set。
- MCP scope pool 对 set 内每个 workspace 建 claim；tool 调用按 `XyToolCtx.workspace` 路由；不在 set 内则拒绝。
- resume 恢复 session 存储的 workspace set；缺失或 trust 变化时重新校验。
- 无 client 时 host 不绑定 workspace set；lifecycle 不变。

## Capabilities

- `server-core`：host workspace set 生命周期。
- `protocol-app`：subscribe `workspaces` 字段。
- `app-tui-trust`：trust 按 workspace set；人类操作入口。
- `infra-mcp`：workspace set 传递到 prepare / pool。

## Impact

host、protocol、TUI client、trust、MCP、session store。

## Further Notes

- 现有 `Subscribe` 只有 `session_id` / `last_seq`；TUI 在 payload 额外发 `cwd`，host `workspace_from_payload` 读取。推荐先正式化 `workspaces`，不新增 handshake。
- 人类入口：TUI slash（如 `/workspace add`）或 host unary；不得做成模型工具。若未来做成工具，必须 human-approval 且审计。
- 1:N 语义：primary 是默认 tool workspace；additional 是受信任集合；mid-session 变更由人类触发。
- 依赖：无。

## Open Questions

- subscribe `workspaces` list vs `workspace` + `additional_workspaces`：推荐 list。
- 人类入口形态：TUI slash / host unary / 两者。
- additional workspace 的 trust 模式与写权限边界。
- workspace set 变更是否需要 session 级审计记录。
