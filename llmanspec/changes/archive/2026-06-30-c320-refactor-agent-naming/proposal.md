---
depends_on: []
---

# c320-refactor-agent-naming

## Why

`agent/` 层当前存在两个结构性病根，让库用户和阅读者难以理解"Agent 到底是什么、从哪进门"。grep + 阅读确认：

### 病根 1：名字与概念重量倒挂

| 类型 | 字段数 | 概念重量 | 实际名字 |
|---|---|---|---|
| 能力聚合体（models + io + tools + 25 字段状态） | 25 | 重 | `AgentSession` ← 被叫"Session"，矮化了 |
| ReAct 驱动器（session + cancel + 循环控制） | 2 | 中 | `AgentLoop` ← 名字尚可但未表达"策略" |
| 空壳（1 字段 `loop_`，纯转发，零行为增量） | 1 | 轻 | `Agent` ← 被叫"Agent"，拔高了 |

`facade.rs` 里的 `Agent` 对 `AgentLoop` **零行为增量**——每个方法都只是 `self.loop_.xxx()` 转发。它存在的全部理由是"名字好听 + re-export 一堆类型"。而 `AgentBuilder` 的 15 个字段是 `AgentSession::new(...)` 15 个参数的构造投影，`build()` 跑完即弃。结果：读者打开 `agent` mod，看到"facade 里只有个空壳 Agent，builder 看上去更像真正的 Agent"，认知摩擦极大。

**命名倒挂的本质**：真正承载能力、可运行时 `set_tools`/`set_permission`/`replace_hooks` 的那个东西（`AgentSession`）被叫成了"Session"；真正的 ReAct 执行体（`AgentLoop`）没有"策略"语义；而最轻的空壳却占用了最泛化的名字"Agent"。

### 病根 2：`AgentSession` 是 god-object

`AgentSession`（25 字段）不只持有状态，还背着 `execute_bash` / `compact_current_session` / `process_prompt` / `export_to_html` / `fork_session` / 模型管理 / 权限检查 / trust 决策…… 一堆**互不内聚的操作**。其中 `export.rs`/`stats.rs` 已是纯函数逻辑层（只差一个持端口的壳），`bash`/`permission`/`trust` 与"能力聚合"毫无内聚理由却挂在对象上。更糟的是**近一半方法是 dead code**（整个 steering 域、`process_prompt`、`check_permission_*` 包装、`save_trust_decision` 等均无生产调用方），是 god-object 膨胀的主因。

### 立场

- **名字倒挂修正**：`AgentSession → Agent`（能力聚合体），`AgentLoop → ReActAgent`（ReAct 策略驱动器），删掉空壳 `Agent`（并入 `ReActAgent`）。`Agent` 这个名字归还给真正承载能力的类型，`ReActAgent` 为未来多 agent 风格家族（plan-execute 等）提前就位。
- **取消 facade 中间层**：`facade.rs` 是纯转发空壳 + re-export hub，取消它，类型直接定义在 `agent/` 下并从 `agent/mod.rs` re-export，减少"打开 mod 看到 facade/builder 两个并列模块、不知从哪进门"的困惑。
- **god-object 彻底拆分**：五块全拆（export / bash / permission / stats / trust），`Agent` 瘦身到只持核心会话上下文 + collaborator。
- **不留兼容 shim**（AGENTS.md 规则 + la14）：改名、拆分、删 dead 一次到位。

### 为何是单一 change 而非多个

改名 / 删空壳 / 取消 facade / 五块拆分 / 清 dead **互为前提**：改名要同步改 builder 目标和所有调用点，删空壳依赖改名，取消 facade 依赖删空壳，拆分依赖改名后的 `Agent` 落位，清 dead 依赖拆分后字段归属明确。拆成多个 change 必然产生中间态兼容 shim（如"暂叫 Agent 但还是 god-object"），违反 AGENTS.md"不留兼容、一步到位"。参照 c265 先例：SDD 的 change 是规划/归档单元，`tasks.md` 天然支持 P0→P5 阶段拆分，每阶段全测试套件绿灯，整个 change 一次 archive。

## What Changes

### P0 — `AgentSession → Agent`（机械改名 + 快照重生成）
1. `agent/session/mod.rs`：`pub struct AgentSession` → `pub struct Agent`；`impl AgentSession` → `impl Agent`。
2. 全局替换所有引用：`agent/builder.rs`、`agent/runtime/react.rs`、`agent/facade.rs`、`agent/session/steering.rs`（`impl super::AgentSession`）、`app/core/composition.rs`、`app/core/driver.rs`、`app/rpc.rs`、`app/server/rest.rs`、`tests/bdd.rs`。
3. 重生成 API 快照：`tests/snapshots/agent_session_api_baseline__*.snap`（`cargo insta review`）。

### P1 — `AgentLoop → ReActAgent` + 删空壳 `Agent` + 合并方法
4. `agent/runtime/react.rs`：`pub struct AgentLoop` → `pub struct ReActAgent`；`impl AgentLoop` → `impl ReActAgent`。
5. 删 `agent/facade.rs` 里的空壳 `Agent`（1 字段 `loop_`，纯转发）；其方法（`run`/`run_with_id`/`abort`/`cancel_token`/`session`/`session_mut`/`set_*`）合并进 `ReActAgent`。
6. 更新 `tests/bdd.rs`（import `AgentLoop` → `ReActAgent`，2 处构造点）。
7. `agent/runtime/mod.rs`：re-export `react::ReActAgent`。

### P2 — 取消 facade 层，类型上提 `agent/mod.rs`
8. 删 `agent/facade.rs` 文件；其 re-export 表面（`AgentBuilder`/`AgentHooks`/`BeforeToolHook`/`XyEventStream`/`XyEvent`）迁到 `agent/mod.rs`。
9. `ReActAgent` / `Agent` 定义直接落在 `agent/`（`react.rs` / `session/mod.rs` 已是它们的物理位置，无需移动文件，只去掉 facade 包装）。
10. 更新所有 import：`crate::agent::facade::{Agent, XyEvent}` → `crate::agent::{Agent, XyEvent}` 等。涉及 `app/core/driver.rs`、`app/rpc.rs`、`app/server/rest.rs`、`app/core/composition.rs`、`agent/builder.rs`、`agent/runtime/react.rs`（test helper 全限定路径）。
11. `agent/mod.rs` 加 `//!` 模块文档：明确"库用户从 `agent/`（mod 级）进门，其余子模块是内部"。

### P3 — 五块拆分（god-object 瘦身）
12. `SessionExporter`（`agent/session/export.rs` 加壳）：持 `export_io: Arc<dyn XyExportIo>`；接收 `&dyn XySessionStore` + sid 参数（纯函数风格，不复制 store）；搬走 `export_to_html`/`export_to_jsonl`/`import_from_jsonl`/`share_as_gist`。`Agent` 持 `exporter: SessionExporter`，方法变 delegate。
13. `BashExecHandler`（`agent/session/bash.rs`，与 `bang.rs` 并列；沿用 as32 命名）：持 `bash_executor`/`bash_cancel`；搬走 `execute_bash`/`record_bash_result`/`abort_bash` + 自由函数 `record_bash_result`。`Agent` 持 `bash: BashExecHandler`（需 `&mut`）。
14. `PermissionGate`（`agent/session/permission.rs`）：持 `permission: Arc<dyn XyPermission>`；暴露 `get_permission() -> Arc<dyn XyPermission>`（react.rs:77 路由契约不变）；搬走 `set_permission`；**删** 3 个 dead `check_permission_read/write/network`。
15. stats 下沉：`get_session_stats` 体挪到 `agent/session/stats.rs::compute(store, sid)`；`Agent` 方法变 1 行 delegate。
16. trust 降级：`save_trust_decision` 降为 `agent/session/trust.rs` 自由函数 `save_trust_decision(cwd, trust_store, trusted)`；`Agent` 不再持此方法。

### P4 — 清 dead code（逐个 grep 核实零调用方后删）
17. 删整个 steering 域（`steer`/`follow_up`/`clear_queue`/`pending_message_count`/`has_pending_steer`/`get_steering_messages`/`get_follow_up_messages`/`drain_queued_messages`/`message_queue`/`message_queue_mut`）+ `steering.rs` 文件。
18. 删 `process_prompt`、`cycle_forward`/`cycle_backward`、`registry_clone`、`set_append_prompt`、`compact_current_session`、`compaction_threshold`/`compaction_settings` 访问器、`register_template`/`register_templates`/`register_command`/`register_skill_commands`、`share_as_gist`、`expand_skill_command`、`set_skills`、`dispose`/`abort`(session 版)。
19. 删散落的 `#[allow(dead_code)]`（逐个核实）。

### P5 — arch_guard 文字 + 文档 + 全 QA
20. `src/tests.rs::arch_guard`：守卫规则不变（扫模块路径前缀，不扫类型名），但更新注释文档里引用 `agent::facade`/`AgentSession` 的文字。
21. `cargo doc --no-deps --all-features` 通过（ReActAgent/Agent/SessionExporter/BashExecHandler/PermissionGate 文档渲染）。
22. 全 QA：`just qa`（fmt check + clippy + nextest + BDD + docs + prek hooks）全绿。
23. 归档前：`llman sdd validate c320-refactor-agent-naming --strict --no-interactive` 通过；`llman sdd archive run c320-refactor-agent-naming`。

## Capabilities

- `agent-runtime`：`AgentLoop → ReActAgent` 改名；删空壳 `Agent`；取消 facade 层，类型上提 `agent/mod.rs`。
- `agent-session`：`AgentSession → Agent` 改名；五块拆分（SessionExporter/BashExecHandler/PermissionGate/stats/trust）；清 dead code。
- `layer-architecture`：la8 facade 措辞 reconcile（HC-2 语义不变）。
- `architecture`：ar02 措辞更新（god-object 拆分落地，"AgentSession facade" → "Agent 能力聚合体"）。

## Impact

- **代码**：`agent/{facade(删),builder,session/mod,session/{export,bash(新),permission(新),stats,trust(新),steering(删)},runtime/react,runtime/mod}`、`app/{core/composition,core/driver,rpc,server/rest,cli/mod}`、`tests/bdd.rs`、`tests/snapshots/agent_session_api_baseline__*.snap`、`src/tests.rs`（注释）。
- **行为**：**运行期行为零变化**。删的全是 dead code（零生产调用方）或纯转发空壳；拆分是内部重组，核心 surface 方法签名保持（rpc.rs/facade→mod/react.rs 调用点零改或最小改）；改名是机械替换。新增的是结构清晰度，不是行为增量。
- **数据**：**零磁盘迁移**。被删的 dead 方法全是进程内临时态；拆出的 collaborator 字段（`export_io`/`bash_executor`/`permission`）原本就在 `AgentSession` 上，只是换了持有者；改名不落盘。session JSONL schema、fork、export 输出字节完全不变。
- **HC 不变量**：**HC-1..HC-6 全部不破**。builder 仍只吃 `Arc<dyn Port>`（不引入新 agent→infra 生产依赖）；HC-2 沿用（`Agent` 构造不持 session_id，la8 语义不变）；HC-4 不受影响；HC-5 **不引入新单实现 trait**——SessionExporter/BashExecHandler/PermissionGate 都是普通 struct，不是 trait。arch_guard 扫的是模块路径前缀（`crate::agent`/`crate::infra`），不扫类型名，改名/拆分/取消 facade 不触发任何守卫失败。
