---
depends_on: [c2780-add-command-execution-class]
---

# bash 派发所有权倒置：交互循环 drain Inline 命令

## Why

c2780 apply 实测（WIP `wip(sdd): c2780 …` 前的诊断）：`run_interactive_bang` 把 bash 派发
钉为 `Box::pin(dispatch(driver, cmd))`（`src/app/tui/effects/bang.rs:49`），该 future
**横跨整个 select 循环持有 `&mut dyn XyDriver`**——tick/input 臂内任何需要 driver 的
effects 调用（含不可变重借）都是 E0499；abort 能碰 driver 正因它先 `drop(dispatch_fut)`
（bang.rs:119-122）。`run_interactive_reload` 同构（`reload.rs:62-63`）。因此
c2780 的 Exec 执行类（atm18）虽已让「Inline=任何循环内立即生效」成为命令的一等声明，
但交互循环的 drain 调度还执行不了它：`/model` 等 Inline 命令在 bang 期间仍只能排队到
abort/结束后的首个 drain。

已排除的小修路径：`execute_session_command(&mut self)` 是 trait 形状（in_process 的
agent 绑定真需要 `&mut`）；drop 后重建 future = 取消并重发 bash unary（双重执行）；
tokio::spawn 不可行（driver 非 Send）。唯一正路是把 bash 派发从「借驱动的 future」
倒置为「拥有型完成接收端」——与 agent 路径 `run() → EventStream` 同模式
（主循环正是因此能在 agent-busy 下自由调度 drain_pending）。

## What Changes

- **派发所有权倒置（bash）**：`XyDriver` 增加 owned 形状的 bash 运行入口
  （一次性 `&mut` 调用返回拥有型完成接收端，镜像 `run() → EventStream`；
  c2760 的输出 sink channel 保持不变），`run_interactive_bang` 的 select 改为
  select 拥有型 receiver 而非借驱动的 dispatch future。
- **交互循环 drain 接入**：bang / reload 交互循环在 borrow 窗口恢复后接入
  `drain_inline_pending`（消费 c2780 `exec_class`：仅 Inline 即时执行，
  Exclusive/Queued 留待循环归还），落 ath45（bang-inline-effect-runs-during-bang /
  exclusive-stays-queued——自 c2780 移交的本场景）。
- **reload 同构**：`run_interactive_reload` 的 `reload_runtime` future 若可同型倒置
  则一并处理；不可同型则只做 bang 并在此注明。

## 非目标

- 不动 Host 侧租约/`Auth::Readonly` 写者期间 `slot.driver` 锁争用（另行立项）。
- 不改 agent 主循环（已正确调度 drain_pending）。
- 不引入 proc-macro / REGISTRY 表宏化（c2780 design §1 的后续重构，独立评估）。

## Capabilities

- `app-tui-host`：ath45 交互循环 drain Inline 命令（自 c2780 移交的规则与场景落地）。
- `app-tui-commands`：无新规则（消费既有 atm18；如实现需要仅补充说明性文字）。

## Impact

- 用户可见：bang / reload 期间 `/model`、`/session-name`、`/history-copy-last`、
  `/theme` 等 Inline 命令即开即用、即关即走。
- 风险：bash 完成语义从 unary 改为接收端，超时/取消路径（30s unary 超时、abort
  带外取消）需要逐一保真迁移；ScriptedDriver / in_process / remote 三实现同改。
