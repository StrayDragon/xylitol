# Tasks — Host workspace set binding

- [x] 1. 协议与 host 入口：`subscribe` `workspaces` 列表、primary 解析、`cwd` 回退、additional 诊断；`Command::Subscribe` 字段；单测。
- [x] 2. 会话绑定：`SessionSlot` workspace set、resume primary 取 header cwd、`SessionHeader`/`SessionManifest` 可选 `additional_workspaces`（formatVersion 7）；serde 单测。
- [x] 3. `new_session` 使用 primary workspace（driver `reload_cwd`），不再用 host 进程 cwd；单测。
- [x] 4. 工具 workspace 成员校验：agent workspace set + `XyToolCtx.workspace` 不在 set 内拒绝；单测。
- [x] 5. Specs 与测试：protocol-app / server-core / agent-runtime 条款；host / runtime 单测与 BDD。
- [x] 6. 门禁：`just fmt` / `just lint` / 相关测试 / `llman-sdd validate c3050-add-host-workspace-binding --strict` / `just qa`。
