# Design: 命令执行类（Exec）声明与交互循环 drain 接入

## 1. Exec 放哪：一张表，不造第二词表

三个候选：

| 候选 | 取舍 |
|---|---|
| A. wire `REGISTRY` 行加 `Exec` 维度（与 `Auth/Idem/Resp/command_backed` 并列） | ✅ 单一属性行已是方法事实源，守卫测试既有先例（UNARY_METHODS 顺序锁）；host 侧未来可直接消费 |
| B. 只做客户端 `exec_class(Command) -> Exec` 纯函数 | 好消费，但与 REGISTRY 漂移无闸 |
| C. PendingSlash 上加每变体方法（像 BusySlashPolicy） | 继续分裂：Command 直派路径（非 slash）拿不到语义 |

**取 A+B 融合**：`Exec` 进 REGISTRY 行（SSOT），`protocol` 层暴露
`exec_class(cmd: &Command) -> Exec`。消费侧的强制分两层吃 Rust 特性：

1. **现在（t1）**：`exec_class` 写成**无通配的穷举 `match`**——新增 `Command` 变体
   不声明执行类直接编译失败，分类由编译器强制，守卫测试只兜 REGISTRY 一致性。
2. **之后（独立重构，不进本 change）**：`macro_rules!` 单行源——每命令一行同时生成
   REGISTRY 行与 match 臂，行表与推导彻底同源，连一致性守卫都可省。暂不做：
   动既有受守卫锁序的表结构，收益不抵本 change 的回归面。

客户端只允许经 `exec_class` 消费，MUST NOT 再散落逐命令特判
（延续 atm16 的收口方向）。

## 2. 分类与初版清单

```rust
pub enum Exec { Exclusive, Inline, Queued }
```

- `Exclusive`：接管会话循环直至完成——`bash`、`prompt`（agent turn）、`reload`。
- `Inline`：任何循环内立即执行、MUST 非阻塞（缓存/本地直出，参照
  `11c70806` 后的 `/model` open 路径）——`get_state`、`get_available_models`、
  `set_model`、`cycle_model`、`set_thinking_level`、`queue_stats`、`get_commands`、
  `loaded_resources`、`append_entry_label`、`set_session_name(_for)`。
- `Queued`：其余（fork/switch/import/export/compact/trust/…），行为与今天一致
  （等循环归还后由 `drain_pending` 处理）。

约定：`Inline` 是「随行生效」的承诺，不是分类箱的兜底；新命令默认 `Queued`，
要 `Inline` 必须满足非阻塞约束（design §4）。

## 3. 交互循环 drain 接入：移交 c2790（apply 阻断记录）

原方案：`run_interactive_bang` / `run_interactive_reload` 的 `tokio::select!` 在 tick 臂内
调用窄入口 `drain_inline_pending(session, driver)`，仅执行声明为 `Inline` 的 pending。

**apply 实测不可行**：两个交互循环的长时 future（`Box::pin(dispatch(driver, cmd))`，
bang.rs:49；`reload_fut = driver.reload_runtime(&cancel)`，reload.rs:62）横跨 select
持有 `&mut dyn XyDriver`，tick 臂内任何 driver 调用（含不可变重借）都是 E0499；
abort 能碰 driver 正因先 `drop(dispatch_fut)`。逃逸路径均否定：

- `execute_session_command(&mut self)` 是 trait 形状，in_process 的 agent 绑定真需要
  `&mut`，改 `&self` 波及全部实现；
- drop 后重建 future = 取消并重发 bash unary（双重执行）；
- tokio::spawn 不可行（driver 非 Send）。

**正路（c2790-invert-bash-dispatch-ownership，已 draft）**：bash 派发倒置为
「一次性 `&mut` 调用返回拥有型完成接收端」（镜像 agent `run() → EventStream`，
c2760 输出 sink channel 不变），循环 select 拥有型 receiver 后 borrow 窗口自然打开，
`drain_inline_pending`（本 change §2 的 `exec_class` 消费）即可接入。
本 change 仅交付 §1 的 Exec SSOT 与消费 API；drain 接入全部移交 c2790。

## 4. Inline 非阻塞约束

`Inline` effect MUST NOT await 远程 unary/HTTP（`/model` open 的缓存先挂载即样板，
`11c70806`）。违反后果与今天相同——浮层/effect 迟到。约束落 specs 规则
（MUST 语句），由 review + PTY E2E 时序断言共同守护；不做运行时强制。

## 5. 测试 seam（复用既有，不新造）

- **BDD/rstest-bdd**（`tests::bdd::`）：交互 bang 循环产品与 harness 共享
  （c715/ath9），ScriptedDriver 下驱动「bang 进行中提交 Inline 命令 → effect
  在 bang 结束前生效」可确定性断言——落 `@executable` 场景的主 seam。
- **PTY E2E**（`tests/tui_e2e`，`just test-tui-e2e`）：真进程时序冒烟
  （bang 中 `/model` 挂载、Esc 即关、/exit 干净），承接 11c70806 已修的两例。
- **守卫单测**：Exec 穷举 ∧ REGISTRY 一致性（protocol 层）。

## 6. 进一步 Notes

- Host 侧 `Auth::Readonly` 读命令在写者期间 `slot.driver` 锁争用（实测 2–30s）
  是独立问题，本 change 不动；Inline 的「立即生效」当前指客户端 drain 调度语义。
  若后续 Host 侧也要消费 Exec（例如 Inline+Readonly 走免锁 fast path），
  REGISTRY 行已就位。
- `try_suppress_stale_esc` 的 overlay 守卫（`11c70806`）与本 change 互补：
  那是「Esc 归浮层」的既有语义修正，这里是「命令何时生效」的声明化。
