# Tasks

## T1 补齐 WorkflowUpdated 逐字段锚测

- [ ] 在 `crates/xylitol-wire/src/event.rs` 新增用例 `workflow_updated_keeps_fields_by_field`：构造含嵌套 `sub_workflow`、Some/None 两态可选字段的 `Workflow`，断言 wire 侧键路径逐字段，再断言反序列化回 `XyEvent::WorkflowUpdated` 载荷相等。
- [ ] 将 `XyEvent::WorkflowUpdated` 加入 `every_domain_variant_round_trips` 样本集，计数断言 `cases.len()` 由 23 改为 24。
- 验证：`cargo nextest run -p xylitol-wire` 全绿；新增用例在删除任一字段断言时失败（手工验证一次）。

## T2 WorkflowUpdated 线协议条款（D1 已定：选项 A）

- [ ] 修 r1698 枚举漂移：「须覆盖全部 XyEvent 变体」的列举补 `WorkflowUpdated`（两案共通）。
- [ ] （D1=A 已定）`llman-sdd spec next-req-id` 分配 rN，`spec add-req` 追加载荷规则块（形态对齐 r1718：领域事件须能经 wire Event 表达、往返保真、未映射端可降级忽略且不 panic），`spec add-scenario` 挂嵌套场景。
- [ ] （D1=A 已定）`tests/bdd` 增加对应 `#[scenario]` 绑定（复用既有 wire 往返 harness seam，不新建 seam）。
- 验证：`llman-sdd validate protocol-app` failed 0；D1=A 已定，`cargo nextest run --all-features --test bdd` 相关场景绿。

## T3 删除 ContextTokenSettlement.generation 死字段

- [ ] 删除 `crates/xylitol-protocol/src/event.rs:110` 字段定义与 :252 测试样本字段。
- [ ] 删除 `crates/xylitol-wire/src/event.rs:100` 硬编码写入与 :525 测试样本字段。
- [ ] 同步 `tests/bdd/steps_protocol.rs:803` 的 BDD 构造样本（去 `generation: 1`）。
- [ ] 同步 `protocol-app` r1698 条款表述（若有引用），确认无残留引用。
- 验证：`cargo nextest run -p xylitol-protocol -p xylitol-wire` 全绿；`cargo nextest run --all-features --test bdd` 绿；`rg 'generation' crates/xylitol-protocol/src/event.rs crates/xylitol-wire/src/event.rs tests/bdd/steps_protocol.rs` 无 settlement 相关命中。

## T4 bridge 家族路由注释导航

- [ ] 在 `crates/xylitol-app-tui/src/bridge/mod.rs` `apply_xy_event` 上方补注释表：事件族 → 家族函数 → 文件 → 渲染落点；标注短路顺序即优先级的不变量。
- 验证：`cargo clippy -p xylitol-app-tui --all-targets` 无新增告警；`just qa-check-uncomment` 不把该注释判为可移除（必要时加 `~keep`）。

## T5 五个 _for_test 方法全部保留并补 doc 注释（已更正）

原 T5「删除零调用点方法」与实测冲突：`mark_run_inflight_for_test` 与 `flush_pending_runtime_for_test` **均有仓内调用点**，见 proposal「事实更正」第 3 条。本 task 改为纯注释补齐。

- [ ] 为 `crates/xylitol-runtime/src/driver/in_process/mod.rs:448/460/473` 补 doc 注释：说明调用方为测试、挂 `#[cfg(test)]` 不可行（跨 crate 调用，cfg 只在被测 crate 自身测试编译时激活）。
- [ ] 为 `crates/xylitol-serve/src/occupancy/host.rs:535/539` 补 doc 注释：同上，并注明调用方为 `tests/bdd/steps_server.rs` 与 `tests/occupancy/assembly_fixture_tests.rs`。
- [ ] 为 `crates/xylitol-serve/src/occupancy/engine.rs:24` 的 trait 方法补 doc 注释。
- [ ] 不改任何方法的可见性，不删除任何方法，不新增 `#[cfg(test)]` 门。
- 验证：`rg 'mark_run_inflight_for_test|flush_pending_runtime_for_test' crates tests` 仍命中原有 8 处测试调用点 + 2 处定义（数量不变）；`cargo nextest run -p xylitol-serve -p xylitol-runtime` 全绿；`just doc` 无新增告警。

## T6 全量门禁

- [ ] `just qa` 全绿。
- 验证：门禁输出附于 tasks 收口记录；`llman-sdd validate c3402-refactor-wire-payload-and-test-seam-hygiene --strict` failed 0。
