# Design

## 逐项深挖决策

### C15-a · WorkflowUpdated 载荷锚测

**现状**：`every_domain_variant_round_trips`（`crates/xylitol-wire/src/event.rs:367-470`）用 23 个样本覆盖域变体，唯独缺 `WorkflowUpdated`。该变体在 `event_to_wire`（:117）与 `event_from_wire`（:254）各自分派到 `workflow_updated_to_wire`/`_from_wire`，二者只是 `serde_json::to_value`/`from_value` 的 `reshape`。载荷 `Workflow`（`crates/xylitol-plugin-sdk/src/session/workflow.rs:105-114`）含 `goal`、`tasks: Vec<WorkflowTask>`、`resume_cursor: Option<String>`、`updated_at`；`WorkflowTask`（:76-101）另有 `depends_on`/`inputs`/`outputs`/`tools`/`risk`/`verification`/`rollback`/`requires_human`/`status`/`output_ref`/`sub_workflow`。

**问题**：`reshape` 依赖 serde derive，任何字段改名、类型变更或 `skip_serializing_if` 调整都不会被现有测试发现。而 `llmanspec/specs/` 无该变体条款，等于载荷形状无任何护栏。

**决策**：
- 新增独立用例 `workflow_updated_keeps_fields_by_field`，断言：顶层 `goal`/`updated_at`/`resume_cursor`；任务 `id`/`description`/`status`/`risk`/`requires_human`；可选字段 `verification`/`rollback`/`output_ref` 的 Some/None 两态；嵌套 `sub_workflow` 一层。断言对象为 wire 侧 `Value` 的键路径（逐字段），再断言反序列化回 `XyEvent` 的载荷相等——对齐既有 `todo_updated_keeps_items_field_by_field`（:562-577）的写法。
- 将 `WorkflowUpdated` 加入 `every_domain_variant_round_trips` 样本集，计数断言由 23 改 24。
- spec：在 `protocol-app` 追加规则块，条款形态对齐 r1718 `todo-updated-wire`（领域事件须能经 wire Event 表达、往返保真、未映射端可降级忽略且不 panic）。

**删除验证**：删掉 `reshape` 通用路径改为手写映射，复杂度不消失而是散到每个变体——说明 `reshape` 在扛事，保留。但「无断言」是真实的护栏缺口，加厚点在于测试而非生产代码。

### C15-b · ContextTokenSettlement.generation 死字段（破坏性评估已定）

**现状**：`Event::ContextTokenSettlement.generation: u64`（`crates/xylitol-protocol/src/event.rs:110`，**无** `#[serde(default)]`）。wire 序列化硬编码 `generation: 0`（`crates/xylitol-wire/src/event.rs:100`）。`XyEvent::ContextTokenSettlement` 只有 `estimate` + `reason`（`crates/xylitol-plugin-sdk/src/settlement.rs:30-34`），`ContextTokenEstimate` 无该字段（`crates/xylitol-plugin-sdk/src/dto/chunk.rs:72-80`）。反序列化侧（`event.rs:218-235`）解构用 `..` 忽略该字段。

**问题**：全链路恒 0，无信息量；线协议多一个必填字段，任何消费方都须携带。

**决策**：删除该字段（protocol 定义、wire 写入、protocol 既有测试样本 `generation: 5` 于 `event.rs:252`、wire 测试样本 `generation: 1` 于 `event.rs:525`）。

**破坏性评估**：字段为必填 `u64`，旧载荷若仍携带该字段，serde 默认忽略未知字段，**不会**导致解析失败；反之新载荷缺失该字段对旧客户端会失败——但本仓无已发布跨版本客户端约束（`PROTOCOL_VERSION` 经 `host.describe` 协商）。按 `llmanspec/AGENTS.md`，字段级演进走可选字段与 `Unknown` 降级，删除恒零字段属净收缩。**已裁定（2026-11-20 用户确认）直接删除**：补充证据——workspace 版本 `0.0.0-pre`（`Cargo.toml:44`）；`EventJournal` 内存环形缓冲不落盘（`crates/xylitol-serve/src/occupancy/ws.rs:23`）；引入提交 `cacec64` 主题与 settlement 无关；实施须同步 BDD 步骤构造（`tests/bdd/steps_protocol.rs:803`）。

### C16 · bridge 家族路由注释导航

**现状**：`apply_xy_event`（`crates/xylitol-app-tui/src/bridge/mod.rs:27-36`）按 `agent → stream → tools → lifecycle` 顺序 `||` 短路。家族实现在 `handlers/` 四文件（`agent.rs` 62 行、`stream.rs` 28、`tools.rs` 84、`lifecycle.rs` 206，`mod.rs` 9 行）。`ToolExecutionEnd` 的完整链路：wire `event.rs:48-56` → serve 分派 → `driver/remote.rs:711` → `host/mod.rs:1465` → `apply_xy_event` → `handlers/tools.rs:24-33` → `apply_tool_end`（:67-84）→ `tool_projection::apply_tool_result_to_entries` → layout 渲染。

**问题**：定位「某事件由谁处理」须通读四个家族文件并注意短路顺序；短路顺序本身是隐式不变量（先匹配者胜，后续家族不再执行）。

**决策**：在 `apply_xy_event` 上方补一张注释表：事件族 → 家族函数 → 文件 → 渲染落点；并显式标注「短路顺序即优先级，同一事件不得被两个家族同时声明」。不改行为、不改函数结构。

**不纳入 spec 的理由**：按 `llmanspec/AGENTS.md`「spec 约束层级」，注释属代码组织，不改变产品可观察行为，不得写成 spec 条款（避免给注释施加 spec 级硬约束）。**待确认项**见 proposal Open Questions。

### C17 · _for_test 可见性收口（已更正，2026-11-20 深挖）

**现状与调用面（实测）**：

| 方法 | 位置 | 仓内调用点 | 处置 |
|---|---|---|---|
| `agent_cwd_for_test` | `crates/xylitol-runtime/src/driver/in_process/mod.rs:448` | `crates/xylitol-kernel/src/occupancy/factory.rs:340`（trait 实现转发）、`tests/occupancy/assembly_fixture_tests.rs:1584`（外部集成测试） | 保留 + doc |
| `tool_names_for_test` | `.../in_process/mod.rs:460` | `crates/xylitol-kernel/src/composition.rs:202,221,240,272,282,338,358,380` | 保留 + doc |
| `system_prompt_for_test` | `.../in_process/mod.rs:473` | `crates/xylitol-kernel/src/bootstrap_tests.rs:48,63,87,90,109,394` | 保留 + doc |
| `mark_run_inflight_for_test` | `crates/xylitol-serve/src/occupancy/host.rs:535` | `tests/bdd/steps_server.rs:3419,3444,4133`、`tests/occupancy/assembly_fixture_tests.rs:1823,1841,1849,1866` | 保留 + doc |
| `flush_pending_runtime_for_test` | `.../host.rs:539` | `tests/occupancy/assembly_fixture_tests.rs:1842` | 保留 + doc |
| `agent_cwd_for_test`（trait） | `crates/xylitol-serve/src/occupancy/engine.rs:24` | 同上外部集成测试 | 保留 |

**原提案断言被推翻**：原 design 称 `mark_run_inflight_for_test` 与 `flush_pending_runtime_for_test` 「仓内零调用点」，实测**均有调用点**。`rg -n 'mark_run_inflight_for_test|flush_pending_runtime_for_test' --glob '!target'` 命中上述 11 处测试调用。

**问题本质**：`#[cfg(test)]` 只在被测 crate 自身测试编译时激活；`xylitol-kernel` / `tests/` 调用 `xylitol-runtime` 与 `xylitol-serve` 的这些方法时，对方以普通依赖编译，方法不存在。故五个方法**都不能**挂 `#[cfg(test)]` 门。

**决策（已定 2026-11-20）**：
- 五个方法全部**保留**，不删除、不挂 cfg 门、不改可见性。
- 每个方法补 doc 注释，注明「测试接缝」性质与调用方位置，使读者知道它不是产品 API。
- 改名去掉 `_for_test` 后缀的方案**否决**：跨 crate 调用点分散在 5 个文件 11 处，改名收益低且与 c3401 的驱动面迁移冲突。

**删除验证**：删掉任一方法，对应测试调用点会编译失败——复杂度在 N 个调用点重新冒出来，说明它们在扛事（提供产品路径不可达的内部状态观测）。这是**测试接缝**，不是纯透传。

**与既有 change 的关系**：`c3401-refactor-runtime-driver-surface-contract-home`（draft，XyDriver 形状单一真值）正处理 runtime 驱动面，本项改名可能与之冲突，故本 change 只做零调用点删除，改名留给 c3401。

## 实施顺序

1. C15-a 测试与 spec 条款（无破坏性）。
2. C15-b 死字段删除（破坏性，独立 commit）。
3. C16 注释导航（纯注释）。
4. C17 doc 注释补齐（纯注释，不删不改可见性）。

四项互不依赖，可任意顺序；C15-b 与 C15-a 同文件（`wire/src/event.rs`），须串行。
