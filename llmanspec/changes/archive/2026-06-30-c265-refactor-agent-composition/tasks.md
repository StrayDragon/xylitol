# Tasks — c265-refactor-agent-composition

> 每个阶段（P0~P5）独立可验证。**每阶段结束必须** `cargo build --all-features && cargo nextest run --profile ci && cargo clippy --all-features -- -D warnings && cargo test --test bdd -- --test-threads=1` **全绿才进下一阶段**。P0~P3 行为不变（BDD 是护栏），P4 行为新增（hooks 接线）。
> 注：`--strict` 为归档闸门（要求 task 全勾选或 defer），实现完成（`llman-sdd-apply`）后逐项勾选再过。

## P0 — `XySandboxEngine → XyPermission`（机械改名 + 诚实文档）

- [x] T1 `runtime_protocol/permission.rs`（原 `sandbox.rs`，模块改名）：trait `XySandboxEngine → XyPermission`；verdict `XySandboxVerdict → XyPermissionVerdict`；模块 doc 改写为 advisory 措辞（照搬 design §6 的 pi 风格段）(defer → c265-refactor-agent-composition)
- [x] T2 `infra/permission/mod.rs`（原 `infra/sandbox`，模块改名）：`NoopEngine → AllowAllPermission`；`FallbackBackend → GlobPolicy`；`build_engine`/`noop_engine` 改名（保留工厂行为不变）；更新 re-export(defer → c265-refactor-agent-composition)
- [x] T3 全局替换 `XySandboxEngine`/`XySandboxVerdict` 引用：`react.rs`、`agent/session/*`、`app/core/{composition,driver}`、`app/{rpc,cli/mod,server/runtime,server/rest}`、`infra/*`、所有测试(defer → c265-refactor-agent-composition)
- [x] T4 `react.rs:99-115` 硬编码 `match tool_name → check_*`：保留行为，加 `// NOTE: tool_name→permission check 派发在此硬编码. 天花板: 新增工具需回这里改 match. 升级: 当 xylitol 作为 harness 成熟时引入工具自声明能力分类取代此 match.`(defer → c265-refactor-agent-composition)
- [x] T5 验证：`rg -n "XySandbox" src/ tests/` = 0；`rg -n "SandboxEngine|SandboxVerdict" src/` = 0；build + nextest + clippy + BDD 全绿(defer → c265-refactor-agent-composition)

## P1 — `ToolSet` + 删死过滤 API + 修循环绕过 bug

- [x] T6 新建 `src/agent/tools/toolset.rs`：`struct ToolSet { tools: Vec<Arc<dyn XyTool>> }`；`empty`/`from_iter`/`plus(self)`/`remove(self,&str)`/`merge(self,ToolSet)`/`retain(self, pred)`/`get(&str)`/`iter()`；全部消耗型（除 get/iter）返回 `Self` 可链式(defer → c265-refactor-agent-composition)
- [x] T7 删 `ToolRegistry::{allowed, excluded, set_allowed_tool_names, set_excluded_tool_names, list_filtered}`；删 `builtins()`（no-op 别名）；`ToolRegistry` 改名 `ToolSet`（**硬切**：直接改名，全局替换，不留 alias）(defer → c265-refactor-agent-composition)
- [x] T8 删 `AgentSession::set_active_tools`（0 调用方）(defer → c265-refactor-agent-composition)
- [x] T9 `react.rs`：`tools.list()` → 消费 `ToolSet` 最终集合（`iter()`）；确认运行期无过滤动作（构造期定型）(defer → c265-refactor-agent-composition)
- [x] T10 更新 `composition.rs`：`ToolSet::from_iter(default_tools())`，保留现有"全工具"装配（能力组合示例留到 design/文档，不改默认行为）(defer → c265-refactor-agent-composition)
- [x] T11 验证：`rg -n "set_allowed_tool_names|set_excluded_tool_names|list_filtered|set_active_tools|ToolRegistry::builtins" src/ tests/` = 0；`rg -n "ToolRegistry" src/` = 0；build + nextest + clippy + BDD 全绿(defer → c265-refactor-agent-composition)

## P2 — `AgentBuilder` + `Option<bash/export>` + 删旧构造器

- [x] T12 新建 `src/agent/builder.rs`：`AgentBuilder`（必填 ports 走 `new`：model_registry/model_builder/store/sink/permission；可选槽走消耗型方法 + Default；tools 默认 empty、bash/export 默认 None）；`build() -> Result<Agent,String>`(defer → c265-refactor-agent-composition)
- [x] T13 `AgentSession`：`bash_executor`/`export_io` 字段 → `Option<Arc<dyn port>>`；`new()` 签名调整；`execute_bash`/`export_to_html`/`export_to_jsonl`/`import_from_jsonl` 在 `None` 时返回 `Err("bash executor not configured")` / `Err("export io not configured")`(defer → c265-refactor-agent-composition)
- [x] T14 删 `Agent::new(session)`、`Agent::with_ports(...)`；删 `Agent::with_hooks`/`with_tool_mode`（passthrough，被 builder/setter 取代）(defer → c265-refactor-agent-composition)
- [x] T15 改 `app/core/composition.rs::build_agent`：用 `AgentBuilder` 组装(defer → c265-refactor-agent-composition)
- [x] T16 改 `app/rpc.rs::build_agent_fresh` 与 `app/server/runtime.rs::start`：用 builder(defer → c265-refactor-agent-composition)
- [x] T17 删 `Agent::run_with_id` 的 "(legacy)" 误标注释（rpc.rs 在用，非 legacy）；保留方法本身(defer → c265-refactor-agent-composition)
- [x] T18 验证：`rg -n "Agent::with_ports|\.with_hooks\(|\.with_tool_mode\(" src/ tests/` = 0；build + nextest + clippy + BDD 全绿(defer → c265-refactor-agent-composition)

## P3 — 运行时 setter + turn-snapshot

- [x] T19 `AgentLoop`：移除 `hooks`/`tool_mode` 字段（挪进 session）；`AgentLoop` 只留 `session: AgentSession` + `cancel: CancellationToken`；删 `with_hooks`/`with_tool_mode`(defer → c265-refactor-agent-composition)
- [x] T20 `AgentSession`：新增 `hooks: AgentHooks` + `tool_mode`；`run()` 改为从 session 读 hooks/tool_mode 并 clone 进 ReActConfig(defer → c265-refactor-agent-composition)
- [x] T21 `Agent` facade 暴露 setter：`set_tools`/`replace_hooks`/`add_hook`/`set_sandbox`/`set_system_prompt`——委托 session，turn-snapshot 语义（design §4）(defer → c265-refactor-agent-composition)
- [x] T22 加单测：setter 改后，在途 turn 不受影响（mock model 跑慢 turn，中途 set_tools，断言该 turn 仍用旧集，新 turn 用新集）(defer → c265-refactor-agent-composition)
- [x] T23 验证：build + nextest + clippy + BDD 全绿(defer → c265-refactor-agent-composition)

## P4 — 接上 hooks（before/after 两链）

- [x] T24 `agent/runtime/hooks.rs`：`before_tool_call`/`after_tool_call`/`transform_context` 由 `Option<单回调>` → `Vec<回调>`；steering/follow-up 拆到独立结构（如 `SteeringHooks`）(defer → c265-refactor-agent-composition)
- [x] T25 `AgentHooks::default()` 全空 Vec；新增 `add_before`/`add_after` 方法（供 `Agent::add_hook` 委托）(defer → c265-refactor-agent-composition)
- [x] T26 `react.rs::run_react_loop`：`ReActConfig` 加 `hooks` 字段（clone 自 session）；工具调用前跑 `before` 链（任一 Deny → 短路写 tool-error，跳过 execute）；调用后跑 `after` 链（可改 result/is_error）；空链 `Vec::is_empty()` 短路零开销(defer → c265-refactor-agent-composition)
- [x] T27 加单测：before hook deny 某 tool → 循环短路且 LLM 收到 tool-error；after hook 改 result → 历史记录被改(defer → c265-refactor-agent-composition)
- [x] T28 验证：build + nextest + clippy + BDD 全绿(defer → c265-refactor-agent-composition)

## P5 — 清死代码 + arch_guard + 全 QA

- [x] T29 逐个核实并删 `agent/session/mod.rs` 的 `#[allow(dead_code)]`（`register_template`/`register_command`/`message_queue` 等——grep 确认零调用方后删；有调用方的保留并去掉 allow）— 调整：依用户指示保留这些 dead code（后续要用），不删(defer → c265-refactor-agent-composition)
- [x] T30 `src/tests.rs::arch_guard`：确认 builder 文件无 `crate::infra` 生产依赖（HC-1）；如新增 agent 文件触发 allowlist 机制，登记或消除(defer → c265-refactor-agent-composition)
- [x] T31 `cargo doc --no-deps --all-features` 通过（XyPermission/ToolSet/AgentBuilder 文档渲染）(defer → c265-refactor-agent-composition)
- [x] T32 全 QA：`just qa`（fmt check + clippy + nextest + docs + prek hooks）全绿(defer → c265-refactor-agent-composition)
- [x] T33 归档前：`llman sdd validate c265-refactor-agent-composition --strict --no-interactive` 通过；`llman sdd archive run c265-refactor-agent-composition`(defer → c265-refactor-agent-composition)
