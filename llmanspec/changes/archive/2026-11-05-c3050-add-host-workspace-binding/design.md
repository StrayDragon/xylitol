# Design — Host workspace set binding

## 1. 协议入口

`subscribe` payload 正式化 `workspaces` 列表：

```json
{ "session_id": "s0", "last_seq": 0, "workspaces": ["/path/to/project"] }
```

- primary = `workspaces[0]`。
- 缺省时回退既有 `cwd` 字段；再回退 host 进程 cwd（兼容入口，不新增 handshake）。
- additional = `workspaces[1..]`；MVP **不启用**：host 记录诊断并只按 primary 物化，不建 claim。
- `Command::Subscribe` 增 `workspaces: Vec<String>`（serde default），保持协议形状与 TUI 发送一致。

## 2. 会话绑定

- `SessionSlot` 增 workspace set（primary + additional）；subscribe / resume 时解析。
- resume 时 primary 取会话存储的 primary（header cwd），不取 client 当前目录；新会话取 client primary。
- `new_session` 用 driver 的 primary workspace（`reload_cwd`），不再用 host 进程 cwd。
- `SessionHeader` 与 `SessionManifest` 增可选 `additional_workspaces`（skip none），formatVersion 保持 7；primary 仍为 header `cwd`，不升版本。
- `Branch.workspace_ref` 仍是分支 backend/commit 引用，不与 workspace set 混用。

## 3. Trust

- 按 workspace 逐个解析 trust；primary 在 subscribe / resume 时重校验并记 `trust_recheck`（c3105 已提供）。
- additional 未启用，不解析其 trust；未来启用时逐项走 trust 门禁。

## 4. 工具路由与拒绝

- 工具执行上下文使用 `XyToolCtx.workspace`。
- agent 持 workspace set；工具执行前若 workspace 不在 set 内，返回 `workspace outside session set` 拒绝，不执行工具。
- MVP 工具默认在 primary 执行；additional workspace 的选择留给后续 change。
- LLM 不得 add / remove / switch workspace：不注册任何相关工具；变更入口只保留人类（TUI slash / host unary，后续 change 落地）。

## 5. 失败与诊断

- primary 路径不存在或不可 canonicalize：回退 host cwd 并产生可读诊断；不得静默绑定错误目录。
- additional 条目：产生 `additional workspace not supported` 诊断；不阻断 primary 绑定。
- resume 找不到 header cwd：回退 client primary 并诊断。

## 6. 测试

- 单测：payload 解析（workspaces / cwd fallback）、primary 选择、additional 诊断、new_session 使用 primary、tool workspace 拒绝、header/manifest serde。
- BDD：subscribe workspaces 与 primary 绑定（host 级）；tool workspace 拒绝（runtime 级）。
