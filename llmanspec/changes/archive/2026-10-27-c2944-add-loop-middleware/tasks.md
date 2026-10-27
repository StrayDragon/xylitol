# c2944 实施任务

Seam（复用既有 harness，不另造）：

| 面 | 复用 |
|---|---|
| 公共接口 | `xylitol-plugin-sdk` 新端口 + `LoopMiddlewareRegistry`；`AgentRuntime::set_loop_middleware` |
| 组合根 | `Kernel::boot` / `boot_ctx`（`crates/xylitol-kernel/src/contract.rs` 既有契约测试） |
| BDD | `tests/bdd/fixtures.rs::AgentState` + `helpers.rs::make_agent` + `bindings_agent_runtime.rs` + `steps_agent_runtime.rs` |
| 观测 | `SpanCollectScope`（`tests/bdd/steps_otel_obs.rs` 既有模式） |
| 门禁 | `just qa`（含 `scripts/check_crate_graph.py`、`check_no_version_names.py`、`check_uncomment.py`） |

BDD 执行入口已配置：`llmanspec/config.yaml` 的 `specs.check_command = cargo test --all-features --test bdd`。

## 垂直切片

- [x] 1. **SPI 词表与注册表**（sdk）：`LoopStage` 七值闭集、`LoopControl` 四值闭集、各阶段 ctx 载荷、`LoopMiddlewareRegistry`（有序注册 / 注销 / 快照）与 `loop_middleware_key()`；`register` 返回卸载句柄。测：同 stage 按 order 排序、同 order 按注册序、注销后快照不含、id 重复拒绝。
- [x] 2. **runtime 七阶段接线（空链基线）**（runtime）：`FrozenRootConfig` 增链快照；`before_turn` / `before_model` / `after_model` / `before_tool` / `after_tool` / `decide_continue` / `after_turn` 执行点接入；`AgentRuntime::set_loop_middleware`。测：链空时既有 `baseline.rs` / react 单测逐事件不变（无新增事件、无 span）。
- [x] 3. **控制语义与安全边界**（runtime）：`stop` / `compact` / `handoff`（未支持 → 稳定错误）落地；`before_tool` deny 在 core permission 之前、改参后 core 重算 target；`decide_continue` 无进展 `continue`/`compact` 硬上限；单次超时与取消竞速。测：改参仍被 permission 拦、deny 不绕过、超上限发 `LoopMiddlewareLimit` 错误后 `AgentEnd`、取消以 aborted 结束。
- [x] 4. **trace 可指认**（runtime obs）：`loop.middleware` span（id / stage / order / control / changed / outcome），仅 `provider_trace_active()` 建。测：`SpanCollectScope` 断言属性可读、链空无 span。
- [x] 5. **组合根接线**（kernel）：`agent-react` 核心插件 `provide(LoopMiddlewareRegistry)`；`Kernel::boot` 取服务交 driver；`XyInProcessDriver::set_loop_middleware`。测：`boot_ctx` 后注册表可读；卸载后服务消失（既有契约测试模式）。
- [x] 6. **官方示例插件**（新 crate + config + 装配）：`xylitol-official-plugin-loop-example`（`before_tool` 只收紧 bash 危险模式、`decide_continue` 自定义停止）；`AppConfig.loop_example` 默认 false；`Builtins` 注册、默认 rows 不装；`check_crate_graph.py` 登记 + config→loop-example peer 例外；`configs/example.yaml` 示例段。测：未配置零装配（无服务、无 span、无行为差）；启用后 deny 生效且卸载零残留。
- [x] 7. **BDD 落地**（agent-runtime）：新增 MUST 规则 + 嵌套场景（七阶段子集、控制枚举、只能更严、死循环上限、trace、示例插件零配置/卸载）；`tests/bdd` 绑定 + 步骤 + fixture 注册入口；`cargo test --all-features --test bdd` 绿，既有 loop 场景不变。
- [x] 8. **文档指针**：`src/AGENTS.md` 中间件边界与「core 终裁」；根 `AGENTS.md` 扩展入口一句；`llmanspec/AGENTS.md` 不改（无新规则）。
- [x] 9. **可见一致性回流**：`fixed_context` 随 before_turn/before_model 刷新；`ToolExecutionStart` 带最终参数；`after_model` 改写先于 `MessageEnd`；中间件故障回合标记非正常结束。BDD：改写参数可见 / 改写助手先于消息结束 / 系统碎片进入压缩上下文。
- [x] 10. **门禁**：`just qa` 全绿；`llman sdd validate c2944-add-loop-middleware --strict --no-interactive` 绿。
