# Tasks: c2710-refactor-driver-command-dispatch

> pre-start。依赖 `c2705`。**需要 Specs landing**（绑定分支后，不在默认分支改 live spec）。单 task 按垂直切片，避免一人改完 trait 另一人改 host 对不上。

> **执行注记（apply 完成）**：
> - 执行器落为 `dispatch::SessionCommandExecutor`（`XyDriver: SessionCommandExecutor` 超 trait）；in-process 直接 match → inherent 方法，remote 单函数 `unary_cmd(Command)` + 缓存副作用。
> - **例外**：`execute_bash` 保留在 `XyDriver`（交互流式面：`chunk_tx` uplink / Esc abort，c665/c669）；`Command::Bash` 仍走执行器（无本地通道）。design §3 的 bash 移出以此为准。
> - `GetQueueStats` 的 serde tag 更名 `queue_stats`（与 registry 主方法名对齐，删 alias）；`DispatchOutcome` 形状不变。

## 1. 闸与合约

- [x] 1.1 `change start`（认领后）并列出将改的 live capabilities（Driver / host unary / wire Command）。
- [x] 1.2 [blocked-by: 1.1] Specs：删除「必须经 `XyDriver::<method>`」的钉法，改为 Command 执行器；删除 alias 字段场景，改为单一 JSON 键。
- [x] 1.3 [blocked-by: 1.2] `llman sdd validate c2710-refactor-driver-command-dispatch --strict --no-check`。

## 2. 执行器 SSOT

- [x] 2.1 [blocked-by: 1.2] 抽出 `execute_session_command(ctx, Command) -> DispatchOutcome`（名字以代码为准），in-process 实现调 Runtime，**不**再经 50 方法 trait。
- [x] 2.2 [blocked-by: 2.1] `dispatch.rs` 改为对该执行器的薄包装或删除重复 match。
- [x] 2.3 [blocked-by: 2.2] `dispatch_session_unary` 除 Prompt/Subscribe/lease 外走执行器；删巨型 method 分支。
- [x] 2.4 [blocked-by: 2.3] `XyRemoteDriver`：会话操作用 `unary(Command)`；删平行方法体。

## 3. 缩 trait

- [x] 3.1 [blocked-by: 2.4] `XyDriver` 按 `design.md` §3 删除已迁走的方法；修 TUI/Print/embed/harness 调用点为 Command 或执行器。
- [x] 3.2 [blocked-by: 3.1] 删除 `command.rs` 全部 `serde(alias)`；更新 registry / 夹具 / OpenAPI 调试文档。

## 4. 验证

- [x] 4.1 [blocked-by: 3.2] Driver + host + dispatch 测试；相关 BDD。
- [x] 4.2 [blocked-by: 4.1] `just qa`；扫 `dispatch_session_unary` 复杂度是否下降。
- [x] 4.3 [blocked-by: 4.2] 交接：注明 `/debug` 与 `cycle_thinking` 仍在（留给 c2740/c2730）。
