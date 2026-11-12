# Design — TUI busy 状态与 ask 槽（2026-11-12 复核后定稿）

## 1. 目标与边界

- busy 单一活动真值；ask 槽不可静默覆盖；流式缓冲单点复位；结算判据去字符串。
- 边界：不改 Esc 语义与效果循环结构（PendingOps lane 机制保留，本 change 不类型化）。

## 2. 决策摘要（2026-11-12 复核后全部定稿）

| 决策 | 内容 | 状态 |
|---|---|---|
| busy 相位 | **host 层位集** `HostBusy { agent_run, bash, reload }` 单一活动真值；`recalc_busy()` 为 Idle/Busy 唯一收口；状态文案保留 bridge 细粒度，仅当文案是活动标签且该活动消失时回落到剩余活动标签 | 定稿 |
| 收口 | end_bash / end_reload / note_user_abort / run 收尾 全走 `recalc_busy()` 按剩余活动决定 Idle | 定稿 |
| ask 槽 | 本地 ask 占用时拒绝（既有）；远端 approval 排队容量 1（`pending_remote`）；溢出 即时以拒绝载荷回应 + 通知条提示 + 记日志 | 定稿 |
| 流式缓冲 | 复核发现 `UiModel::clear_streaming_buffers()` 已是单一复位入口；缺口是 lifecycle / session_tree 两处绕过。修复 = 全部走该入口，**不新建 struct**（避免字段访问大面积迁移） | 定稿（修正） |
| 结算判据 | wire 事件 reason 为 `String`；在 app-tui 边界解析为 `ContextTokenSettlementReason` 并匹配枚举，**不改线协议形状** | 定稿（修正） |
| PendingOps lane | 本 change 不类型化（design §1 保留 lane 机制，另单） | 定稿（范围裁定） |

## 3. busy 位集

`HostBusy` 三个 bool 位；begin 置位、end 清位后统一 `recalc_busy()`：
- 位集空 → `UiPhase::Idle` + status None。
- 位集非空 → `UiPhase::Busy`；status 为 None 或为活动标签（Working / Running bash / Reloading）时 取最高优先级活动文案（Reloading > Working > Running bash），bridge 的细粒度文案（Drafting reply / Compacting / Retry …）保留不动。
- reload 中 end_bash 不再误置 Idle（位集仍含 reload）；note_user_abort 只清 agent/bang，reload 位保留。

## 4. ask 槽占用策略

- `AskHostGateway`：`pending`（当前槽）+ `pending_remote`（容量 1）+ `notices`（溢出通知）。
- `push_from_server`：`pending` 空闲 → 直接入槽；占用 → 入 `pending_remote`；`pending_remote` 已占 → 即时以拒绝载荷回应新请求（approval 回 `approved=false`，ask 回空答案），push 通知文案并 `log::warn!`。
- `take_pending`：先取 `pending`，空则提升 `pending_remote`。
- host `poll_ask_host` 每次 drain `take_notices()`，经通知条呈现。

## 5. 测试边界（seam）

- harness：reload 中 bash 结束状态栏仍 Reloading；reload 中 abort 仍 Reloading；reload 结束且有 run 时回 Busy。
- ask：远端 approval 与本地 ask 并发（排队 1、溢出拒绝 + 通知）；`take_pending` 提升顺序。
- 流式缓冲：abort 与 session_tree 重建都走同一复位入口（断言字段全清）。
- 结算：reason 枚举解析（turn_settled / after_compaction 置位，其它不置位）。

## 6. 风险与回滚

- 回滚：busy / ask / buffers / settlement 四片独立。

## 7. 实施前复核（2026-11-12）

| 结论 | 复核 | 证据 |
|---|---|---|
| busy 多源：run/bash/reload 三 bool + UiModel.phase | 成立 | host/mod.rs:96-98、448-450、505-507、573-575；968、975、1451、1455 |
| reload 中 end_bash 误置 Idle | 成立 | host/mod.rs:586-593 仅回查 `run_active`，未查 reload |
| note_user_abort 不动 reload | 成立 | host/mod.rs:596-604 清 bash/run，reload 位保留但 model 相位置 Idle |
| set_busy_status 任意事件置 Busy | 成立（bridge 多处写 status；recalc 须保留细粒度文案） | bridge/model.rs:603；handlers/agent.rs、tools.rs、lifecycle.rs、stream.rs |
| ask 单槽静默覆盖 | 成立 | ask_host.rs:71-79 直接 `*slot = Some(...)`；本地 ask 在 120-124 已拒绝 |
| 流式缓冲「六文件手工清」 | **部分不成立**：`clear_streaming_buffers()` 已是单一入口（model.rs:457-463）；绕过点为 lifecycle.rs:92-95、session_tree.rs:23-27 | 同上 |
| 结算判据字符串 | 成立 | lifecycle.rs:153-157（reason: String）；host/mod.rs:984-992 匹配字面量 |
| PendingOps 14 lane | 未复核；design §1 已裁定保留机制、本 change 不类型化 | — |
