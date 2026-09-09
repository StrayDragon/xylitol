# Tasks: c2710-refactor-driver-command-dispatch

> pre-start。依赖 `c2705`。**需要 Specs landing**（绑定分支后，不在默认分支改 live spec）。单 task 按垂直切片，避免一人改完 trait 另一人改 host 对不上。

## 1. 闸与合约

- [ ] 1.1 `change start`（认领后）并列出将改的 live capabilities（Driver / host unary / wire Command）。
- [ ] 1.2 [blocked-by: 1.1] Specs：删除「必须经 `XyDriver::<method>`」的钉法，改为 Command 执行器；删除 alias 字段场景，改为单一 JSON 键。
- [ ] 1.3 [blocked-by: 1.2] `llman sdd validate c2710-refactor-driver-command-dispatch --strict --no-check`。

## 2. 执行器 SSOT

- [ ] 2.1 [blocked-by: 1.2] 抽出 `execute_session_command(ctx, Command) -> DispatchOutcome`（名字以代码为准），in-process 实现调 Runtime，**不**再经 50 方法 trait。
- [ ] 2.2 [blocked-by: 2.1] `dispatch.rs` 改为对该执行器的薄包装或删除重复 match。
- [ ] 2.3 [blocked-by: 2.2] `dispatch_session_unary` 除 Prompt/Subscribe/lease 外走执行器；删巨型 method 分支。
- [ ] 2.4 [blocked-by: 2.3] `XyRemoteDriver`：会话操作用 `unary(Command)`；删平行方法体。

## 3. 缩 trait

- [ ] 3.1 [blocked-by: 2.4] `XyDriver` 按 `design.md` §3 删除已迁走的方法；修 TUI/Print/embed/harness 调用点为 Command 或执行器。
- [ ] 3.2 [blocked-by: 3.1] 删除 `command.rs` 全部 `serde(alias)`；更新 registry / 夹具 / OpenAPI 调试文档。

## 4. 验证

- [ ] 4.1 [blocked-by: 3.2] Driver + host + dispatch 测试；相关 BDD。
- [ ] 4.2 [blocked-by: 4.1] `just qa`；扫 `dispatch_session_unary` 复杂度是否下降。
- [ ] 4.3 [blocked-by: 4.2] 交接：注明 `/debug` 与 `cycle_thinking` 仍在（留给 c2740/c2730）。
