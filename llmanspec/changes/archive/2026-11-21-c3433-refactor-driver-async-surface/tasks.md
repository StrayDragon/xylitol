# Tasks：驱动同步面 async 化

> 依赖：无。needs_specs_change: false。
> seam：runtime/kernel 单测 + 全量 BDD 回归地板 + occupancy 测试组。

- [x] T1: trait 面改造——`set_capability` / `clear_capabilities` / `take_mcp_gate_notice` 改为 `async fn`；`set_bash_run_sink` 按设计点 2 实测后决定是否保留同步。
- [x] T2: in-process 实现——`XyInProcessDriver` 对应方法改 async，内部逻辑不变。
- [x] T3: 包装与端侧——`OccupancyDriver`（kernel）、`XyRemoteDriver`（app-tui）同步改造。
- [x] T4: 测试替身——`ScriptedDriver` / `StubDriver` / `OAuthDriver` 与 BDD step 定义同步改造。
- [x] T5: 调用点改造——`set_capability` 5 处、`clear_capabilities` 2 处、`take_mcp_gate_notice` 5 处加 `.await`；`set_bash_run_sink` 8 处零改动（设计点 2）。固有同步 `set_capability` / `clear_capabilities` 删除，函数体并入 trait impl。
- [x] T6: 回归——全量 BDD 1240 场景 + occupancy 48 例 + kernel/runtime 单测全绿；`just qa` 实跑。
- [x] T7: 收尾——`llman-sdd validate --strict` 通过；结论回写 c3432 design 第 6 点。
