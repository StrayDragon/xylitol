# src/ 分层架构（代码架构 SSOT）

本文件是 `src/` 代码架构的**单一真值源**：分层地图、分层不变量、各层职责、应用面状态、seam 定义。同一事实不在此文件之外重复——子目录 `AGENTS.md`（如 `src/app/tui/AGENTS.md`）与 skills（`write-surface`/`audit-dead-code`/`write-tui`）只做路径引用。全局规则见根 `AGENTS.md`。

`xylitol` 是单 crate：薄编排核心（`agent/`）架在大型运行时域（`infra/`）之上，可插拔应用面（`app/`）说同一种线协议（`protocol/`），底层是纯领域词汇（`domain/`）与 agent↔infra 边界 traits（`runtime_protocol/`）。

## 分层不变量（normative）

下列依赖方向由 `src/tests.rs::arch_guard` 强制（代码是真值）。每条标注对应的守卫测试。

- **组合根集中装配**（arch_guard `composition_root` 类约束）：只有组合根允许同时 import `agent` 与 `infra`——`app/core/composition.rs`（主组合根，唯一集中装配 `Agent`），以及次级组合根 `app/cli/mod.rs`、`app/server/subcommand.rs`，在构造期注入 adapter。
- **agent 层不依赖 infra**（守卫：`arch_guard::agent_does_not_import_infra_in_production`）：`agent` 永不 import `infra` 具体类型；只依赖 `domain` + `runtime_protocol`。
- **infra 层不依赖 agent**（守卫：`arch_guard::infra_does_not_import_agent`）：`infra` 永不 import `agent`；只依赖 `domain` + `runtime_protocol`。
- **domain 零内部依赖**：`domain` 不依赖任何 crate 内模块。
- **runtime_protocol 只依赖 domain**。
- **应用面走 seam、不 reach 内部**（部分守卫：`arch_guard::app_driver_does_not_import_infra`；session/runtime 内部 reach 靠 review）：应用面禁止 reach into `agent::session::*`/`agent::runtime::*`/`infra::*`，只从 `crate::agent`（mod 级）与 `crate::app::core`（`Driver`/`composition`）import。所有应用面共享同一 seam：`composition::build_agent` → `Driver::run(prompt)` → `XyEvent` 流 → 该面渲染；seam 不够就扩 seam（`Driver` trait / `XyEvent` 枚举），不绕过。新增/改造应用面的方法论见 `write-surface` skill。

依赖方向图：

```text
app → agent → runtime_protocol → domain
  ↓     ↑
  └──── infra ───────────────────┘
protocol ───────────────────────→ domain
```

## 各层职责

- `domain/` — 纯领域词汇 + 错误 + serde 类型（`XyEvent` 在 `lifecycle.rs`）。零 crate 内依赖。
- `runtime_protocol/` — agent↔infra 边界 traits：`XyModel`/`XyModelBuilder`/`XyStream`、`XyTool`/`XyToolCtx`/`XyToolExecutionMode`、`XySessionStore`、`XyEventSink`、`XyBashExecutor`、`XyExportIo`、`XyPermission`、`XyResourceLoader`、`XySecretResolver`、`XyTrustStore`。只依赖 `domain/`。
- `infra/` — 运行时域，实现 `runtime_protocol/` 的 ports：`provider/`（LLM adapter，`adapter/` 含 anthropic/openai-completions/openai-responses + `factory`）、`tools/`（内建工具实现）、`session/`、`process/`、`config/`（`value.rs` 密钥解析）、`event/`、`hooks/`、`mcp/`、`resource/`、`trust/`、`permission/`、`settings/`、`git/`、`clipboard/`、`image/`、`bash_exec/`、`browser/`、`fs_watch/`、`update/`、`export/`（`StdExportIo`）。
- `agent/` — 薄编排：`runtime/`（ReAct 循环 `react.rs`、`event.rs`、`hooks.rs`、`permission_router.rs`、`retry.rs`）、`session/`（`Agent` 可插拔能力聚合体）、`model/`（`registry`+`manager`+`resolver`+`manifest`）、`tools/`（`ToolSet`+`definition`，实现不在本层）、`compaction/`、`prompt/`（`system`/`commands`/`templates`）、`builder.rs`（`AgentBuilder`）。mod 级 re-export 是公共入口：`AgentBuilder`/`ReActAgent`/`Agent`/`AgentHooks`/`BeforeToolHook`/`XyEventStream`/`XyEvent`。交互层只从 `crate::agent::*` import。
- `protocol/` — client↔core 线协议 SSOT：`command.rs`（`Command` 枚举）、`event.rs`（`Event` 枚举）、`transport.rs`。传输无关。
- `app/` — 应用面 + 跨面 seam（`core/`）。**应用面状态**：`cli/print.rs` ✅（print 模式，默认，一次性 prompt → 流式 stdout）；`cli/mod.rs` 组合根 + mode 分发 + 调 `app::core::bootstrap`（c336，装配集中化）；`server/` 🟡（feature 门控 REST/WS，`subcommand.rs` 管生命周期，经 `bootstrap` 装配）；`tui/` ✅（inline REPL，`Viewport::Inline` + Driver seam，c340 落地 + c341 依赖瘦身 + c360 渲染 harness/RenderedLine seam + c365 流式 mutable-last-line/组件化 + c336 `/model` 经共享 dispatch，见 `src/app/tui/AGENTS.md`）；`gui.rs` 🔴 空占位。rpc 已移除（c336：零消费者 + 784 行违反 spec ip4「薄传输」，remote 用例由 server WS/REST 承担）。`core/bootstrap.rs`（共享装配）+ `core/dispatch.rs`（共享 Command 分发，经 Driver trait）+ `core/composition.rs`（注入 ports）+ `core/driver.rs`（`InProcessDriver`/`RemoteDriver`）是跨面 seam，其余面经它们，不 reach agent/infra 内部。落地顺序 print → server → TUI，逐个端到端跑通再开下一个，禁止并行铺骨架。子目录细则见 `src/app/tui/AGENTS.md`。

## 跨层测试与守卫

- 架构守卫：`src/tests.rs::arch_guard`（拦截 `agent ↔ infra` 互引）。
- BDD：`tests/features/*.feature` + `tests/bdd.rs`（rstest-bdd），harness 在 `tests/support/`，快照在 `tests/support/snapshots/`。
- 回归：`tests/regression/{issue号}-{简述}.rs`。
- 分层历史与设计依据：`llmanspec/changes/archive/<变更>/design.md`。
