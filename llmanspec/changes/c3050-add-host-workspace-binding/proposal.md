---
depends_on: [c3105-fix-harden-trust-store]
title: "Host workspace set binding from client"
---

## Why

Host 当前启动即绑定 cwd；`new_session` 用 host 进程 cwd；多 client / 多 workspace 无法隔离。MCP per-workspace 与 trust 需要 workspace 从 client 传入。一个 session 可以绑定一个 workspace，也可能绑定多个受信任 workspace（1:N，非无限；多数 1:1）。

## What Changes

- host boot 不再要求 workspace；workspace set 可选。
- client 接入时传 workspace set：primary workspace + additional trusted workspaces。
- 推荐在 `subscribe` 中正式化 `workspaces` 字段（list）；现有 `cwd` 作为 primary 的兼容入口。
- primary workspace = TUI/其他客户端启动目录；resume 时 = session 存储的 primary。
- additional workspace 只能由人类添加/移除：TUI slash 或 host unary；不得暴露为 LLM 可调用工具。
- **人类入口暂不做**：只保留 host unary / TUI slash 插入点；后续可能加。
- workspace set 的 mid-session 变更只允许人类操作；LLM 不得自行 add / remove / switch。
- trust 按 workspace set 逐个解析；新加入的 workspace 走 trust 门禁。
- additional workspace 重新确认机制暂不做；agent 工具有绕过风险，后续讨论。
- host 按 session 绑定 workspace set；`new_session` 用 primary workspace，不用 host cwd。
- materialize driver、MCP pool、tool catalog 使用 session 的 workspace set。
- MCP scope pool 对 set 内每个 workspace 建 claim；tool 调用按 `XyToolCtx.workspace` 路由；不在 set 内则拒绝。
- resume 恢复 session 存储的 workspace set；缺失或 trust 变化时重新校验。
- 无 client 时 host 不绑定 workspace set；lifecycle 不变。

## Trust store 安全

- 威胁：`trust.json` 当前由 `fs::write` 写入，默认权限（实际 0644）；同用户 agent / 进程可改写并自信任 workspace，绕过 trust 门禁。
- 缓解：trust store 0600 原子写；lock 0600；HMAC key 在 agent dir；人类确认；不得暴露为模型工具；resume 重新校验；审计。
- 完整性：HMAC key 在 agent dir，同用户 agent 可读；只提高门槛，不能对抗同用户任意代码执行。
- 真边界：不可信 workspace 用 OS sandbox / 独立用户 / 容器；trust store 是 advisory。
- 独立 change：`c3105-fix-harden-trust-store`。

## Capabilities

- `server-core`：host workspace set 生命周期。
- `protocol-app`：subscribe `workspaces` 字段。
- `app-tui-trust`：trust 按 workspace set；人类操作入口。
- `infra-mcp`：workspace set 传递到 prepare / pool。
- `domain-security`：trust store 加固。

## Impact

host、protocol、TUI client、trust、MCP、session store。

## Further Notes

- 现有 `Subscribe` 只有 `session_id` / `last_seq`；TUI 在 payload 额外发 `cwd`，host `workspace_from_payload` 读取。推荐先正式化 `workspaces`，不新增 handshake。
- 人类入口：TUI slash（如 `/workspace add`）或 host unary；不得做成模型工具。暂不实现，保留插入点。
- 1:N 语义：primary 是默认 tool workspace；additional 是受信任集合；mid-session 变更由人类触发。
- 依赖：`c3105-fix-harden-trust-store`（trust store 加固）；安全后门未处理前不得落地 workspace trust。

## Open Questions

- subscribe `workspaces` list vs `workspace` + `additional_workspaces`：推荐 list。
- 人类入口形态：TUI slash / host unary / 两者；暂不做。
- additional workspace 的 trust 模式与写权限边界；重新确认暂不做。
- workspace set 变更是否需要 session 级审计记录。
- trust store 加固是否先行：见 `c3105-fix-harden-trust-store`。
