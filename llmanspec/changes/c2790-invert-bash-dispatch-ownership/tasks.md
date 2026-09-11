# Tasks

> seam（已与用户确认）：BDD/rstest-bdd ScriptedDriver 共享交互 bang 循环
> （`tests::bdd::`，HostPump 模式 + 挂起 bang + 定时注入流）为主验收 seam；
> PTY E2E（`just test-tui-e2e`）作真进程时序冒烟；protocol 守卫单测兜 SSOT。

## t1: `XyDriver::bash_run`——三实现 + trait 默认

- `proto.rs`：声明 `async fn bash_run(&mut self, command, exclude_from_context)
  -> Result<BashRun, XyDriverError>`（owned 完成接收端；默认 unsupported）。
- remote：克隆 host Arc + session id 进 owned async block，`host.unary("bash", …)`；
  超时/sink/downlink 语义逐一保真（design §2 表）。
- in_process：经 bang runtime 句柄 owned 化 `execute_bash`（abort → cancelled 收尾）。
- harness：`aborted: AtomicBool` → `Arc<AtomicBool>`；`bash_run` 保真
  `hang_bash_until_abort` 与 `bash_calls` 记录。
- 验证：`cargo test -p xylitol --lib`（既有 bang/harness 测试全绿，行为零变化）。

## t2: bang 循环重构 + drain 三件套 + BDD 场景（[blocked-by: t1]）

- `commands.rs`：`SlashAllowances` 增 `exec: Exec` 列（单表穷举，见 design §4；
  不新增平行 match）——Inline v1 名单 = OpenModels / Theme / HistoryCopyLast
  （已逐条核实非阻塞），SetModel / SessionName 列 Queued（remote 写者租约勘误）。
- `host/mod.rs`：`put_slash`。
- `effects/mod.rs`：`drain_inline_pending`（slash 分流；模型确认不入 drain，
  留主循环）。
- `run_interactive_bang`：派发臂改 `bash_run` receiver；tick 臂接 drain
  （design §3 键序；abort/完成/sink/超时逐字节保真）。
- BDD：`bang-inline-effect-runs-during-bang`（atm18）、
  `exclusive-stays-queued`（ath45，样例 `/session-export`）。
- 验证：`cargo test --lib --all-features tests::bdd::`。

## t3: PTY 收紧回 atc23 原契约（[blocked-by: t2]）

- `pty_product_fake_busy_model_list_keeps_running_lead`：bang 不结束即挂载
  （`→ * fake`）→ `Running` lead 在视口 → Esc 关浮层 → Esc 取消 bang →
  `/exit` 干净退出。
- 验证：`just test-tui-e2e` 30/30。

## t4: 全门禁收尾（[blocked-by: t3]）

- `just qa`（fmt/lint/nextest/live-provider）+ `just test-tui` +
  `llman sdd validate c2790-invert-bash-dispatch-ownership --strict --check`。
- reload 不动的决定回写 proposal Further Notes（若实现中发现可倒置，升级为
  本 change 内可选项并在 tasks 勾选说明）。
