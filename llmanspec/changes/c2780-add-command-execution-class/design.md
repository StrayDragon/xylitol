# Design: 命令执行类（Exec）声明与交互循环泵

## 1. Exec 放哪：一张表，不造第二词表

三个候选：

| 候选 | 取舍 |
|---|---|
| A. wire `REGISTRY` 行加 `Exec` 维度（与 `Auth/Idem/Resp/command_backed` 并列） | ✅ 单一属性行已是方法事实源，守卫测试既有先例（UNARY_METHODS 顺序锁）；host 侧未来可直接消费 |
| B. 只做客户端 `exec_class(Command) -> Exec` 纯函数 | 好消费，但与 REGISTRY 漂移无闸 |
| C. PendingSlash 上加每变体方法（像 BusySlashPolicy） | 继续分裂：Command 直派路径（非 slash）拿不到语义 |

**取 A+B 融合**：`Exec` 进 REGISTRY 行（SSOT），`protocol` 层暴露
`exec_class(cmd: &Command) -> Exec`（经既有 command_backed 映射推导），守卫测试锁定
「逐 Command 枚举穷举 ∧ 与 REGISTRY 行一致」。客户端只允许经 `exec_class` 消费，
MUST NOT 再散落逐命令特判（延续 atm16 的收口方向）。

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

## 3. 泵改造：bang/reload select 加一个窄臂

`run_interactive_bang` / `run_interactive_reload` 的 `tokio::select!` 在 tick 臂内
（复用既有 16ms tick 节奏）调用窄入口 `drain_inline_pending(session, driver)`：

- 仅处理 `take_slash`/pending 中「effects arm 将派发的 Command 属 `Inline`」的项；
- `Exclusive`/`Queued` 原样留在 pending（`drain_pending` 归还后处理，语义不变）；
- 主循环 `drain_pending` 不变（全量）。

 WHY tick 臂而非独立 select 臂：不引入额外唤醒源，bang 输出 chunk 泵节奏不被
 effects 打断；Inline effect 本身非阻塞，卡 tick 一拍以内。

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
  是独立问题，本 change 不动；Inline 的「立即生效」当前指客户端泵语义。
  若后续 Host 侧也要消费 Exec（例如 Inline+Readonly 走免锁 fast path），
  REGISTRY 行已就位。
- `try_suppress_stale_esc` 的 overlay 守卫（`11c70806`）与本 change 互补：
  那是「Esc 归浮层」的既有语义修正，这里是「命令何时生效」的声明化。
