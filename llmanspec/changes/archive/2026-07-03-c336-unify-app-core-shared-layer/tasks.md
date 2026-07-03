# c336-unify-app-core-shared-layer — Tasks

> 关键设计决策见 design.md：共享 dispatch 通过**扩展 Driver trait**（而非引入新抽象或让 dispatch 直接持 ReActAgent）来承载 Command 执行语义。这样 InProcessDriver/RemoteDriver 各自实现命令方法，共享 dispatch 是纯分派器。

## 阶段 1：共享装配（bootstrap）

- [x] 1.1 新增 `src/app/core/bootstrap.rs`：定义 `BootstrapInput` / `BootstrappedAgent` / `BootstrapError` / `pub fn bootstrap(input) -> Result<BootstrappedAgent>`，封装 cli/mod.rs::run() Step 1-5 的全部前置装配（config load → registry build → trust resolution → resource 发现 → compaction settings → permission → build_agent → model select → register_prompt_commands）。
- [x] 1.2 `src/app/core/mod.rs` 注册 `pub(crate) mod bootstrap;` 并更新模块注释（说明 bootstrap 是共享装配点）。
- [x] 1.3 `cli/mod.rs::run()`：把 Step 1-5 内联装配替换为 `bootstrap(input)?` 调用，保留 mode 分发（rpc/print/tui）原样。`cargo build`。
- [x] 1.4 回归 print：lib 测试 482 全过 + BDD 87 全过（装配链路无回归；无 API key 故未跑真实 LLM，但 print.rs 未改动，装配产出经测试覆盖）。
- [x] 1.5 `server/runtime.rs`：把 `build_agent` 调用替换为 `bootstrap(...)`，**补回** context_files / append_system_prompt 发现（移除 runtime.rs:104 的 headless 跳过注释）。ServerConfig 同步移除装配字段（model_registry/system_prompt/compaction_*），只保留传输字段。
- [x] 1.6 回归 server：server BDD 2/2 通过；server 现经 bootstrap 与 print 同装配路径，resource 发现（AGENTS.md/context_files/append_system_prompt）自动生效。

## 阶段 2：扩展 Driver trait + 共享分发（dispatch）

- [x] 2.1 扩展 `Driver` trait（design.md 选项 A）：补齐命令执行方法 `select_model` / `cycle_model` / `current_model` / `available_models` / `compact` / `get_state` / `export_html` / `export_jsonl` / `import_jsonl` / `set_thinking_level` 等非 WS 专属变体对应的方法。
- [x] 2.2 `InProcessDriver` 实现新增方法（内部调 `self.agent.select_model(...)` 等）。
- [x] 2.3 `RemoteDriver`（`#[cfg(feature="server")]`）占位实现（命令路由待 server 补齐，返回 not-implemented）。
- [x] 2.4 新增 `src/app/core/dispatch.rs`：定义 `DispatchOutcome` / `DispatchError` / `pub async fn dispatch(driver, cmd)`，纯 match 分派器（Command → Driver 方法调用，无状态）。Prompt/Quit/WS 变体由调用方处理。
- [x] 2.5 `src/app/core/mod.rs` 注册 `pub(crate) mod dispatch;`。
- [x] 2.6 ~~rpc.rs 切换~~ → **改为移除 rpc**（见 design.md「rpc 模式的移除」）：实现期调查发现 rpc 零外部消费者、零真实测试覆盖、现状 784 行重逻辑违反 spec ip4「该是薄传输」。决策移除而非重写。
- [x] 2.7 移除 rpc：删 `src/app/rpc.rs`（784 行）+ cli/mod.rs 的 `--rpc` flag 与 dispatch 分支 + `tests/features/rpc.feature` + bdd.rs 的 RpcTest/handle_rpc_command/rpc scenario + Cargo.toml 的 rpc feature + app/mod.rs 的 mod 声明。
- [x] 2.8 `tui/commands.rs`：移除 `/model` stub，改为调用共享 dispatch（`/model` → `Command::CycleModel`，`/model <id>` → `Command::SetModel`）。
- [x] 2.9 验证 TUI `/model` 经共享 dispatch 实际切换（commands 单测 4/4 通过，含 model_no_arg_cycles / model_with_arg_sets）。

## 阶段 3：架构校验与回归

- [x] 3.1 `arch_guard` 通过（4/4）：bootstrap/dispatch 属 app::core seam，无 agent↔infra 违规；SessionStats 经 driver.rs re-export 避免 tui 引用 crate::agent::session。
- [x] 3.2 全量测试：lib 480 全过 + BDD 85 全过（rpc 2 scenario 移除后 87→85）。
- [x] 3.3 fmt + clippy：`cargo fmt --check` 干净；`cargo clippy`（lib+bins，lint gate）零 warning。
- [x] 3.4 docs：`cargo doc --no-deps --all-features` 生成成功。
- [x] 3.5 dispatch 单测 5/5 + tui commands 单测 4/4 通过。

## 反降级护栏自检（archive 前必过）

- [x] print 与 tui 实际调用 `bootstrap(...)`（cli/mod.rs 经 bootstrap 构造 InProcessDriver）。
- [x] server 实际调用 `bootstrap(...)`，且补回 context_files/append_system_prompt（runtime.rs 移除 headless 跳过）。
- [x] ~~rpc 实际调用共享 `dispatch(...)`~~ → rpc 已移除（零消费者 + 违反 spec ip4，见 design.md）。
- [x] tui `/model` 经共享 dispatch 实际切换模型（tui/commands.rs 经 shared_dispatch，单测验证）。
- [x] print 输出回归通过（lib+BDD 覆盖装配链路，print.rs 未改）。
- [x] ~~rpc 命令行为回归通过~~ → rpc 移除（原 dispatch 无真实测试覆盖，移除即消除违反 ip4 的重实现）。
- [x] 共享 dispatch 不含 WS 专属逻辑（Subscribe/ApproveTool/AnswerQuestion 留 server/ws.rs，dispatch 返回错误要求调用方处理）。
- [x] arch_guard 不报违规。
