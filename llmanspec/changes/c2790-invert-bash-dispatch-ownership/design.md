# Design: bash 派发所有权倒置与交互循环 drain 接入

## 1. 为什么只能倒置（逃生路径全否）

| 候选 | 否定理由 |
|---|---|
| `execute_session_command(&mut self)` 改 `&self` | trait 形状；in_process 的 agent 绑定（`bind_session_or_err` 等）真需要 `&mut`，改签名波及全部实现与调用点 |
| drop 后重建 dispatch future | 取消并重发 bash unary = 双重执行 |
| `tokio::spawn` 派发 | driver 非 Send |
| tick 臂内不可变重借 `&*driver` | `Pin<Box<Future>>` 横持 `&mut`，重借同样 E0499 |

正路与 agent 路径同构：`run()` 一次 `&mut` 完成所有准备（clone Arc 句柄），返回
拥有型 stream（remote.rs:887-917 的 `async_stream::stream!` + `Box::pin`）。
bash 的「准备」= sink 已由 `set_bash_run_sink` 独立挂载；派发体只需要
host client Arc + session id + 载荷。

## 2. `XyDriver::bash_run` 形状

```rust
/// 交互 bash（c2790）：一次性 `&mut` 调用，返回拥有型完成接收端。
/// 输出 chunk 仍经 `set_bash_run_sink` 喂送（downlink / 本地 sink），
/// 与本 future 解耦；超时（host client 30s）在 future 内部自裁。
async fn bash_run(
    &mut self,
    command: &str,
    exclude_from_context: bool,
) -> Result<BashRun, XyDriverError>;
```

`BashRun = Pin<Box<dyn Future<Output = Result<XyBashResult, XyDriverError>> + Send>>`
（async_trait 自动装箱；与 `EventStream` 的 BoxPin 惯例一致）。trait 默认实现
`Err(XyDriverError::unsupported(...))`——dispatch.rs 的测试 stub 不受迫。

三实现：

| 实现 | bash_run 体 |
|---|---|
| remote | 克隆 `self.host`（Arc）+ session id + 载荷进 owned async block，`host.unary("bash", …)`；chunk 仍由 downlink→sink 喂送；超时/取消在 `http_ws` 传输层，语义不变 |
| in_process | `BangExecHandler` 已全 Arc 内部态（`executor: Option<Arc<dyn XyBashExecutor>>`、`cancel: Arc<Mutex<…>>`，bang_exec.rs:17-24，注释明示 abort 无需 `&mut`）——克隆句柄进 owned async block；abort 时 `bang.abort()` 杀进程树，future 以 cancelled 收尾 |
| harness（ScriptedDriver） | `aborted: AtomicBool` → `Arc<AtomicBool>`；`bash_run` 返回轮询该 Arc 的 owned future，保真 `hang_bash_until_abort` / `bash_calls` 记录 |

## 3. `run_interactive_bang` 重构

```rust
let mut bash_run = driver.bash_run(&bash.command, bash.exclude_from_context).await?;
let mut bash_run = Box::pin(bash_run);          // 拥有型，不再借 driver
loop {
    tokio::select! {
        biased;
        result = &mut bash_run => break Some(result),          // 完成语义不变
        chunk = chunk_rx.recv(), if !chunks_done => { … },      // 输出泵不变
        _ = ticker.tick() => {
            session.step(HostEvent::Tick)?;
            effects::drain_inline_pending(session, driver).await;   // ← 借用已自由
        }
        maybe = input.next() => { …take_abort 同今天：break None… }
        maybe_agent = …同今天…
    }
}
drop(bash_run);
driver.set_bash_run_sink(None);
if abort_requested { driver.abort(); session.note_bash_cancelled(); … }  // 逐字节保真
```

- abort 时序与今天一致：Esc 臂只 latch + break；`driver.abort()` 在 borrow 释放后
  调用（区别仅：今天 drop 的是 dispatch_fut，明天 drop 的是拥有型 receiver）。
- 极端路径：receiver 被 drop 而 host 侧 bash 仍在跑（如未来新增的放弃式退出）——
  与今天 drop(dispatch_fut) 的行为等价，由带外 abort / host 超时兜底。
- `drain_inline_pending` 阻塞上限 = Inline effect 自身（非阻塞承诺，atm18）；
  chunk 臂 biased 在前，输出泵优先级不降。

## 4. drain 侧：加列，不另开表（含 apply 前复核结论）

1. **`SlashAllowances` 增列 `exec: Exec`**（commands.rs）：该结构体文档本就声明
   「Single exhaustive table——future gates add a column here, do not fork
   parallel matches」，执行类正是这样的门禁列；`slash_allowances` 单表穷举同时
   回答 busy 准入与执行类，不再出现第二张平行 match 表（避免 Repeated
   Switches，用户裁定「发现即处理」，在本 change 内完成）。
2. **Inline 列名单（v1 收紧）**：`OpenModels`、`Theme`、`HistoryCopyLast`。
   逐条核实：OpenModels = remote 走本地缓存直出（remote.rs:1253 拦截，不触
   wire）；Theme = 纯本地 `reload_themes`；HistoryCopyLast = 共享本地剪贴板
   模块（remote/in_process 的 `copy_text_to_clipboard` 均委托
   `super::clipboard`，无 unary，remote.rs:1184-1189 / in_process:420-425）。
   **`SetModel` / `SessionName` 列 `Queued`**：二者是真实远程 unary
   （`Auth::Writer`），bang 的 bash unary 在场时会撞写者租约排队——Inline 执行
   将冻结整个循环（含 chunk 输出），违反 atm18 非阻塞承诺；其 busy-Allow 语义
   不变（可提交），执行等循环归还。此收紧为本 change 对 c2780 名单的勘误。
3. `host/mod.rs`：`pub fn put_slash(&mut self, slash: PendingSlash)`（放回
   `pending.slash`；Allow-but-Queued 命令在 bang 期间不被丢弃的唯一保障）。
4. `effects/mod.rs`：`pub async fn drain_inline_pending(session, driver)`——
   `take_slash` → 查 `slash_allowances(&slash).exec`，Inline 则
   `slash::handle_slash`，否则 `put_slash`。**模型浮层的确认动作
   （`take_pending_model_select` → SetModel/SetThinkingLevel）不进 drain**：
   确认是真实 unary，留在主循环 drain（bang 结束后生效）——v1 明确边界。

## 5. Specs 计划（重落自 c2780 移交）

- `app-tui-commands.feature`：atm18 `@executable` 场景
  `bang-inline-effect-runs-during-bang`（ScriptedDriver，bang 进行中注入
  `/model`+Enter → 循环结束前浮层已挂载）。
- `app-tui-host.feature`：ath45 `@human` 规则 `interactive-loop-pumps-inline`
  （重落）+ `@executable` 场景 `exclusive-stays-queued`——Queued 样例为
  `/session-export`（busy-Allow 且 Queued；`/session-fork` 是 busy-Reject，
  提交即被拒不入队，作样例不成立——c2780 apply 勘误）。
- BDD 绑定：复用 `steps_app_tui_host.rs` 的 HostPump 模式 +
  `set_hang_bash_until_abort` + 定时注入流（`esc_stream` 改造型：先注入
  `/model`+Enter 再 Esc）。
- PTY E2E（t3）：`pty_product_fake_busy_model_list_keeps_running_lead`
  收紧回原始 atc23 契约——bang 不结束，`/model` 提交后浮层即挂载（`→ * fake`），
  `Running` lead 保持 raw 流在视口，Esc 关浮层 → Esc 取消 bang → `/exit` 干净。

## 6. Further Notes

- reload：`reload_runtime(&mut self, &cancel)` 两实现需 `&mut`（remote 写
  `gate_notice_consumed`；in_process 触及 settings/mcp 装配），不可同型倒置。
  后续若要 reload 期间也可 drain，路径 = reload_runtime 拆「发起（&mut 一次）+
  拥有型进度接收端」，收益/复杂度比低（reload 有软闸），暂不立项。
- `run_interactive_bang` 重构后 `dispatch_fut`/`Command::Bash` 经 dispatch 的
  交互路径退役，但 `dispatch(Command::Bash)` 本身保留（host 侧 unary 语义不变，
  供非交互调用方）。
- REGISTRY 宏化（c2780 design §1 第 2 层）保持独立，不搭车。
