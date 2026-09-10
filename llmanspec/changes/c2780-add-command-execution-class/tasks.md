# Tasks

## t1: Exec 类 SSOT——REGISTRY 维度 + `exec_class` 推导 + 守卫测试

- `protocol/wire/registry.rs`：`MethodEntry` 增 `Exec` 维度；按 design §2 初版清单
  为 REGISTRY 全部行标注（`Exclusive`：bash/prompt/reload；`Inline`：get_state、
  get_available_models、set_model、cycle_model、set_thinking_level、queue_stats、
  get_commands、loaded_resources、append_entry_label、set_session_name(_for)；
  其余 `Queued`）。
- `protocol` 层新增 `exec_class(&Command) -> Exec`（经 command_backed 映射推导）。
- 守卫单测：逐 `Command` 变体穷举 → `exec_class` 有定义且与 REGISTRY 行一致；
  REGISTRY 行数/顺序守卫沿用既有模式。
- 验证：`cargo test -p xylitol protocol`（守卫绿）。

## t2: bang/reload 交互循环泵 `Inline`（[blocked-by: t1]）

- `effects` 层新增窄入口 `drain_inline_pending(session, driver)`：仅处理 effects arm
  派发 Command 属 `Inline` 的 pending（slash 为主）；`Exclusive/Queued` 留 pending。
- `run_interactive_bang` / `run_interactive_reload` 的 tick 臂接入该入口；
  主循环 `drain_pending` 不变。
- BDD/harness 测试（seam：tests::bdd，共享交互 bang 循环 c715/ath9）：
  - `@executable` atm18 场景 bang-inline-effect-runs-during-bang：ScriptedDriver
    bang 进行中提交无参 `/model` → bang 结束前模型列表已挂载。
  - `@executable` ath45 场景 exclusive-stays-queued：bang 进行中提交 `Queued`
    类命令（如 /session-fork）→ bang 结束前 MUST NOT 执行、归还后照常执行。
- 验证：`cargo test --lib --all-features tests::bdd::`。

## t3: `/model` / `/thinking` 走 Inline 生效路径 + PTY 时序冒烟（[blocked-by: t2]）

- 检查 `OpenModels`/`SetModel`/`SetThinkingLevel` effects arm 在 Inline 路径下的
  非阻塞约束（缓存先挂载为样板；不得引入新的 await unary）。
- PTY E2E（`tests/tui_e2e`，`just test-tui-e2e`）：bang 进行中无参 `/model` →
  bang 不结束时列表即挂载（`→ * fake`）；Esc 即关；`/exit` 干净退出。
  复用/收紧 `pty_product_fake_busy_model_list_keeps_running_lead`。
- 验证：`just test-tui-e2e` 全绿。

## t4: reload 循环同臂 + 收尾（[blocked-by: t2]）

- `run_interactive_reload` tick 臂接入 `drain_inline_pending`（若 reload 软闸与
  Inline 冲突，以 reload 取消优先，Inline 顺延——写进 harness 断言）。
- `just qa` 全绿；`llman sdd validate c2780-add-command-execution-class --strict`。
