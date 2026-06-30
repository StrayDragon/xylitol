---
depends_on: []
---

# c265-refactor-agent-composition

## Why

`Agent` 的构造与能力装配当前是"能跑但表意错误"的状态。grep + 阅读确认了五处结构性问题：

1. **假 legacy 死代码**。`Agent::new(session)`（`facade.rs:39`，注释标 "(legacy)"）**全仓零调用方**；唯一的真实构造路径是 `Agent::with_ports(...)`，而它只被 `app/core/composition.rs:69` 一处调用。没有迁移在进行，"legacy"注释是误导。

2. **14 参位置构造函数**。`Agent::with_ports` 带被 `#[allow(clippy::too_many_arguments)]` 压制的 14 个位置参数。`composition.rs` 已经用 `BuildAgentOptions` 结构体分好组，却在边界立刻解构成 14 个位置参数——分组在传入处就丢了。

3. **过滤子系统是死的，且被 ReAct 循环绕过**（最关键）。`ToolRegistry` 实现了完整的 `set_allowed_tool_names`/`set_excluded_tool_names`/`list_filtered`，`AgentSession::set_active_tools` 也在——但三者**全仓零调用方**。更糟的是 `react.rs:128-135` 跑循环时用的是 `tools.list()`（返回全部），**完全绕过 `list_filtered()`**。即"建好但从未通电"，且即使设置了过滤运行时也被忽略。

4. **`AgentHooks` 从未被读取**。`AgentLoop.hooks` 字段能被 `with_hooks` 写入，但 `self.hooks` **从未被读取**，5 个回调（`before_tool_call`/`after_tool_call`/`transform_context`/`get_steering_messages`/`get_follow_up_messages`）**全仓零调用点**，`ReActConfig` 里也没有 hooks 字段。你能 set，循环跑起来完全无视它。

5. **`XySandboxEngine` 名字误导，表里不一**。它实际是进程内的、咨询性的 glob 过滤器（`FallbackBackend` 做字符串匹配，循环"自觉不调"被拒工具），既非 OS 级真隔离，也非外部 sandbox 的适配器。叫 `Sandbox` 会让人误以为是安全边界——这恰好是 pi 文档明确反对的："A partial in-process sandbox would be easy to misunderstand as a security boundary"。

本次重构确立 **Agent 装配的组合优先**立场，并诚实命名权限层：

- **Agent 用 builder 装配**：必填 port（model + store + sink + permission）走构造函数，其余能力槽（tools / prompt / hooks / bash / export / sandbox / compaction）走消耗型链式方法，带 Default。
- **经典 agent 只剩最小内核**：tools 默认空、bash/export 默认 `None`，纯对话 agent 不背无用槽。
- **能力组合靠正交维度的单元操作，不靠预设**：撤回 `AgentMode` enum 与 `Capability` 分类（YAGNI），只给 `ToolSet` 的 `retain`/`plus`/`remove`/`merge` 单元操作，判断逻辑交调用方写。探索/RO/plan 模式 = `ToolSet`（构造期定型）+ `XyPermission`（咨询兜底）的协同结果，不是某个旋钮的值。
- **运行时可改，turn 间快照生效**：`Agent` 暴露 `set_tools`/`set_hooks`/`add_hook`/`set_sandbox`/`set_system_prompt`，下一轮 `run()` 自动生效（因 `run()` 已在 turn 开头 clone 快照），支持 app/tui 的 plan/edit 模式快速切换。无锁。
- **接上断线的 hooks**：`AgentHooks` 改 before/after 两链，`react.rs` 真正调用它们。
- **`XySandboxEngine → XyPermission`**：诚实标注为咨询性护栏、非安全边界；真隔离留给未来的 tool-routing 模式。

### 为何是单一 change 而非多个

builder / ToolSet / 运行时 setter / hooks 接线 / 权限改名**互为前提**：builder 消费 `ToolSet` 的形状、setter 依赖 session 持有这些槽、hooks 接线和 ToolSet 都改 `react.rs`、权限改名也改 `react.rs` 的派发。拆成多个 change 必然产生中间态兼容 shim（如"builder 暂时仍吃 ToolRegistry，下个 change 再换"），违反 AGENTS.md"不留兼容、一步到位"。参照 c260 先例：SDD 的 change 是规划/归档单元，`tasks.md` 天然支持 P0→Pn 阶段拆分，每阶段全测试套件绿灯，整个 change 一次 archive。

## What Changes

### P0 — `XySandboxEngine → XyPermission`（机械改名 + 诚实文档）
1. `runtime_protocol/permission.rs`（原 `sandbox.rs`，模块一并改名）：trait `XySandboxEngine → XyPermission`、verdict `XySandboxVerdict → XyPermissionVerdict`。
2. `infra/permission/`（原 `infra/sandbox/`，模块改名以免误导）：`NoopEngine → AllowAllPermission`、`FallbackBackend → GlobPolicy`；工厂与 re-export 改名。
3. 模块文档照搬 pi 措辞：**advisory in-process gate，非安全边界**；xylitol 不做 sandbox，只做兼容性检测。
4. 全局替换所有 `XySandbox*` 引用（`react.rs`、`session`、`composition`、`driver`、`cli`、`server`、测试）。

### P1 — `ToolSet` + 删死过滤 API + 修循环绕过 bug
5. `agent/tools/toolset.rs`（新建）：`empty`/`from_iter`/`plus`/`remove`/`merge`/`retain`/`get`，构造期定型，无运行期过滤。
6. 删 `ToolRegistry::{allowed, excluded, set_allowed_tool_names, set_excluded_tool_names, list_filtered, builtins}` 与 `AgentSession::set_active_tools`。
7. `react.rs` 改为消费最终工具集（消除"`.list()` 绕过过滤"的 bug 类——过滤在构造期完成后，运行期无过滤动作可供绕过）。
8. `react.rs:99-115` 的硬编码 `match tool_name` → 标 `// NOTE:` 锁定，留作 harness 成熟后再引入工具自声明能力分类。

### P2 — `AgentBuilder` + `Option<bash/export>` + 删旧构造器
9. `agent/builder.rs`（新建）：必填 ports（`ModelRegistry` + `model_builder` + `store` + `sink` + `permission`）走 `new`，可选槽（tools 默认 empty、prompt、hooks、bash 默认 None、export 默认 None、sandbox、compaction、max_iterations、cwd）走消耗型方法；`build() -> Result<Agent, String>`。
10. `AgentSession::bash_executor`/`export_io` → `Option<...>`；`execute_bash`/`export_to_*`/`import_from_jsonl` 在 `None` 时返回 `"not configured"` 错误。
11. 删 `Agent::new(session)`、`Agent::with_ports(...)`、`Agent::with_hooks`/`with_tool_mode`（passthrough）。
12. 改三个调用点（`composition.rs`、`rpc.rs`、`server/runtime.rs`）到 builder；删 `run_with_id` 的误标 "(legacy)" 注释（rpc 在用）。

### P3 — 运行时 setter + turn-snapshot
13. 把 `hooks`/`tool_mode` 字段从 `AgentLoop` 挪进 `AgentSession`（消除重复状态）；`AgentLoop` 只留 `session` + `cancel`。
14. `Agent` facade 暴露 `set_tools`/`set_hooks`/`add_hook`/`set_sandbox`/`set_system_prompt`；语义：改的是 session 内槽，下个 `run()` 重新 clone 快照生效，在途 turn 不受影响（无锁）。

### P4 — 接上 hooks（before/after 两链）
15. `AgentHooks`：`before_tool_call`/`after_tool_call` 由 `Option<单回调>` 改 `Vec<回调>`（可 `add_hook` 叠加）；steering/follow-up（消息注入，非工具拦截）拆到独立结构。
16. `react.rs` 的 `run_react_loop` 在工具调用前跑 `before` 链（任一 Deny 即短路返回 tool-error），调用后跑 `after` 链（可观察/改结果）；空链零开销。

### P5 — 清死代码 + arch_guard + 验证
17. 删 `ToolRegistry::builtins`、`AgentSession::set_active_tools`、`session/mod.rs` 散落的 `#[allow(dead_code)]`（逐个核实零调用方后删）。
18. `src/tests.rs::arch_guard`：确认 builder 只吃 `Arc<dyn Port>`（HC-1），无新 agent→infra 生产依赖。
19. 全 QA：`just qa`（fmt + clippy + nextest + BDD + docs）。

## Capabilities

- `agent-runtime`：builder 取代旧构造器；运行时 setter + turn-snapshot；hooks 真正被调用 + 改两链。
- `agent-session`：bash/export 降 `Option`；能力态归 session 持有。
- `tool-system`：`ToolSet` 单元操作；删死过滤 API；修循环绕过。
- `security-policy`：trait 改名 `XyPermission`；advisory 非安全边界的诚实标注。

## Impact

- **代码**：`runtime_protocol/permission.rs`、`infra/permission/`、`agent/{facade,builder,session,runtime/react,runtime/hooks,runtime/permission_router,tools/*}`、`app/{core/composition,core/driver,rpc,cli/mod,server/runtime}`、测试。
- **行为**：运行期行为不变（删的都是死代码/被绕过的路径）；新增 builder/setter/hooks 接线为能力增量。
- **数据**：**零磁盘迁移**。被删的 `active_tools`/`allowed`/`excluded` 全是进程内临时态，不在 session JSONL 里持久化；`ToolSet`/`Option<bash>`/`AgentMode` 都是构造期参数，不落盘。fork/export/session schema 不变。
- **HC 不变量**：HC-1（builder 只吃 port，不破）；HC-2（facade 不持 session_id，沿用）；HC-4（新工具无需改 loop——硬编码 sandbox 派发已标 NOTE 待 harness 成熟）；HC-5（**不引入新单实现 trait**——builder/ToolSet 是普通结构，`XyPermission` 是改名非新增）。
