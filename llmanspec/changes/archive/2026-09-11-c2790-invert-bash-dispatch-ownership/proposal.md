---
depends_on:
- c2780-add-command-execution-class
branch: sdd/c2790-invert-bash-dispatch-ownership
base_sha: afaeac85ff091730d571ae93e22a034421baaeac
checkpointed: true
checkpoint_sha: d073fc49ce41ca95f426a43eb9a53ac055f03eb5
---

# bash 派发所有权倒置：交互循环 drain Inline 命令

## Why

c2780 apply 实测：`run_interactive_bang` 把 bash 派发钉为
`Box::pin(dispatch(driver, cmd))`（`src/app/tui/effects/bang.rs:49`），该 future
**横跨整个 select 循环持有 `&mut dyn XyDriver`**——tick/input 臂内任何需要 driver 的
effects 调用（含不可变重借）都是 E0499；abort 能碰 driver 正因它先 `drop(dispatch_fut)`
（bang.rs:119-122）。因此 c2780 的 Exec 执行类（atm18）虽已让
「Inline=任何循环内立即生效」成为命令的一等声明，泵循环结构上还执行不了它：
`/model` 等 Inline 命令在 bang 期间仍只能排队到 abort/结束后的首个 drain。

深挖后确认的可行正路（三条逃生路径已否定，见 design §1）：把 bash 派发从
「借驱动的 future」倒置为「一次性 `&mut` 调用返回**拥有型完成接收端**」——与 agent
路径 `run() → EventStream` 同模式（remote `run()` 一次 `&mut` 后克隆 Arc 句柄进
`async_stream::stream!`，remote.rs:887-917；主循环正是因此能在 agent-busy 下自由
调度 `drain_pending`）。

支撑事实（本次深挖逐一核实）：

- remote `unary_cmd(&self)` 本就是 `&self`（remote.rs:799，Arc 内部态）——`&mut`
  纯属 trait 形状，owned 化无数据阻力；
- bash 输出 chunk 经 downlink→sink 喂送（remote.rs:315），与 unary 完成 future
  **天然解耦**，倒置后输出泵节奏不变；
- 30s 超时在 host client 传输层（`host_client/http_ws.rs:88`），位于 owned future
  内部，语义自保；
- in_process `execute_bash(&self)` 已是 `&self`（in_process/mod.rs:567），abort 走
  `bang.abort()`（mod.rs:307-309）带外杀进程树；
- harness `aborted: AtomicBool` 是裸字段（harness.rs:43），owned 化需升 `Arc`。

## What Changes

- **`XyDriver::bash_run`（派发所有权倒置）**：一次性 `&mut` 调用，返回拥有型
  完成接收端（`Pin<Box<dyn Future>>` 风格，与 `EventStream` 的 BoxPin 惯例一致，
  经 async_trait 自动装箱）；c2760 的 sink channel 喂送路径不变。三实现同交付：
  remote（克隆 host Arc + session id 进 owned future）、in_process（bang runtime
  句柄）、harness（`aborted` 升 `Arc<AtomicBool>`）。trait 提供默认 `unsupported`
  实现，测试 stub 不受迫。
- **`run_interactive_bang` 重构**：select 的派发臂改 poll 拥有型 receiver；
  tick 臂接入 `drain_inline_pending`（消费 c2780 `exec_class`：仅 Inline 即时
  执行，Exclusive/Queued 经新增 `HostSession::put_slash` 原样放回，归还后由
  主循环 `drain_pending` 处理）。abort 时序保真：Esc → break → `drop` receiver
  → `driver.abort()` 带外杀树，与今天逐字节一致。
- **`slash_exec_class(&PendingSlash) -> Exec`**（commands.rs，穷举）：
  `OpenModels/SetModel/SessionName/HistoryCopyLast/Theme → Inline`；
  `Exit → Exclusive`（提交时即 quit，不入 drain）；其余 `Queued`
  （含 busy-Allow 但重型的 SessionDump/Export/Compact/OpenMcp/OpenSessionResume）。
- **reload 决定：本 change 不动**（bang-only 交付，预案兑现；完整说明见
  Further Notes，后续路径 design §6）。

## 非目标

- 不动 `run_interactive_reload` 的泵结构（见下 Further Notes）。
- 不动 Host 侧租约/`Auth::Readonly` 写者期间 `slot.driver` 锁争用（另行立项）。
- 不改 agent 主循环（已正确调度 drain_pending）。
- 不引入 proc-macro / REGISTRY 表宏化（c2780 design §1 的后续重构，独立评估）。
- 不改 c2770 的 interrupted 提示投影。

## Further Notes（apply 后回写）

- **reload 保持 bang 期间不 drain**：`reload_runtime(&mut self, &cancel)` 两个
  实现均需 `&mut`（remote 写 `gate_notice_consumed`；in_process 触及
  settings/mcp 装配），不可同型倒置；且 reload 有软闸（`try_reload_input`）
  与较短窗口，收益/复杂度比低。实现期未发现可倒置路径，维持提案预案；
  后续路径 = 「发起（&mut 一次）+ 拥有型进度接收端」，暂不立项（design §6）。

## Capabilities

- `app-tui-host`：ath45 交互循环 drain Inline 命令（规则重落 +
  `exclusive-stays-queued` 可执行场景；Queued 样例用 busy-Allow 的
  `/session-export`，非 busy-Reject 的 `/session-fork`）。
- `app-tui-commands`：atm18 可执行场景 `bang-inline-effect-runs-during-bang`
  （自 c2780 移交）。

## Impact

- 用户可见：bang 期间 `/model`、`/session-name`、`/history-copy-last`、`/theme`
  即开即用、即关即走；PTY `busy_model_list_keeps_running_lead` 可回归原始
  atc23 契约（忙碌中挂载浮层且状态 lead 保持视口）。
- 风险：bash 完成语义从借驱动 unary 改为接收端，超时（host client 30s）、abort
  带外取消、sink chunk 三条路径需逐一保真迁移（design §4 逐条列保护措施）；
  三驱动实现同改，BDD/PTY 双 seam 验证。
