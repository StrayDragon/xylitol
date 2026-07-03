# c336-unify-app-core-shared-layer — Tasks

> 关键设计决策见 design.md：共享 dispatch 通过**扩展 Driver trait**（而非引入新抽象或让 dispatch 直接持 ReActAgent）来承载 Command 执行语义。这样 InProcessDriver/RemoteDriver 各自实现命令方法，共享 dispatch 是纯分派器。

## 阶段 1：共享装配（bootstrap）

- [ ] 1.1 新增 `src/app/core/bootstrap.rs`：定义 `BootstrapInput` / `BootstrappedAgent` / `BootstrapError` / `pub fn bootstrap(input) -> Result<BootstrappedAgent>`，封装 cli/mod.rs::run() Step 1-5 的全部前置装配（config load → registry build → trust resolution → resource 发现 → compaction settings → permission → build_agent → model select → register_prompt_commands）。
- [ ] 1.2 `src/app/core/mod.rs` 注册 `pub(crate) mod bootstrap;` 并更新模块注释（说明 bootstrap 是共享装配点）。
- [ ] 1.3 `cli/mod.rs::run()`：把 Step 1-5 内联装配替换为 `bootstrap(input)?` 调用，保留 mode 分发（rpc/print/tui）原样。`cargo build`。
- [ ] 1.4 回归 print：`cargo run -- "Hello!"` 与归档前的输出字节级对比（建议先存 baseline：`cargo run -- "Hello!" > /tmp/print-before.txt 2>&1` 在 1.3 前抓）。
- [ ] 1.5 `server/runtime.rs`：把 `build_agent` 调用替换为 `bootstrap(...)`，**补回** context_files / append_system_prompt 发现（移除 runtime.rs:104 的 headless 跳过注释）。
- [ ] 1.6 回归 server：启动 server，对一个含 AGENTS.md 的 cwd 跑 prompt，确认 resource 被发现（system prompt 含 AGENTS.md 内容）。这是行为变更（非纯重构），需显式验证。

## 阶段 2：扩展 Driver trait + 共享分发（dispatch）

- [ ] 2.1 扩展 `Driver` trait（design.md 选项 A）：补齐命令执行方法 `select_model` / `cycle_model` / `current_model` / `available_models` / `compact` / `get_state` / `export_html` / `export_jsonl` / `import_jsonl` / `set_thinking_level` 等非 WS 专属变体对应的方法。
- [ ] 2.2 `InProcessDriver` 实现新增方法（内部调 `self.agent.select_model(...)` 等）。
- [ ] 2.3 `RemoteDriver`（`#[cfg(feature="server")]`）实现新增方法（内部发 REST 请求）。当前 RemoteDriver 是 dead_code，本步让它真正实现命令路径（为想法 2 remote 模式铺路）。
- [ ] 2.4 新增 `src/app/core/dispatch.rs`：定义 `DispatchOutcome` / `DispatchError` / `pub async fn dispatch(driver: &mut dyn Driver, cmd: Command) -> Result<DispatchOutcome, DispatchError>`，是纯 match 分派器（Command → Driver 方法调用，无状态）。
- [ ] 2.5 `src/app/core/mod.rs` 注册 `pub(crate) mod dispatch;`。
- [ ] 2.6 `rpc.rs::dispatch()`：替换为对共享 dispatch 的调用。rpc 保留：id 提取、`Event::Response{id,..}` 包裹、Subscribe/ApproveTool/AnswerQuestion 的拒绝（WS 专属，留 server/ws.rs）。rpc 的 RpcState 仍管「按命令重建/缓存 agent」，但重建后包进 InProcessDriver 再调共享 dispatch。
- [ ] 2.7 回归 rpc：`cargo test rpc -- --test-threads=1`，各命令行为不变。
- [ ] 2.8 `tui/commands.rs`：移除 `/model` stub，改为调用共享 dispatch（`/model` → `Command::CycleModel`，`/model <id>` → `Command::SetModel`）。
- [ ] 2.9 验证 TUI `/model` 实际切换：`RUST_LOG=debug cargo run --features tui`，`/model` 应切换并在下一轮 prompt 生效。

## 阶段 3：架构校验与回归

- [ ] 3.1 `arch_guard` 通过：`cargo test arch_guard`。确认 bootstrap/dispatch 属 app::core seam，无 agent↔infra 违规；RemoteDriver 实现新方法不引入 infra 依赖（经 reqwest，不 import crate::infra）。
- [ ] 3.2 全量测试：`just test`（含 BDD）。
- [ ] 3.3 fmt + clippy：`just fmt && just lint`。
- [ ] 3.4 docs：`cargo doc --no-deps --all-features` 不报错。
- [ ] 3.5 全套 QA：`just qa`。

## 反降级护栏自检（archive 前必过）

- [ ] print 与 tui 实际调用 `bootstrap(...)`（非仅新增模块）。
- [ ] server 实际调用 `bootstrap(...)`，且补回 context_files/append_system_prompt。
- [ ] rpc 实际调用共享 `dispatch(...)`。
- [ ] tui `/model` 经共享 dispatch 实际切换模型（非 stub）。
- [ ] print 输出回归通过。
- [ ] rpc 命令行为回归通过。
- [ ] 共享 dispatch 不含 WS 专属逻辑（Subscribe/ApproveTool/AnswerQuestion）。
- [ ] arch_guard 不报违规。
