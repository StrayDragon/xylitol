# Design — TUI 远程驱动链路状态（2026-11-12 复核后定稿）

## 1. 目标与边界

- 断链 / 重同步 / 致命三态显式化；fatal 可观测且可恢复；连接死亡单一载体。
- 边界：不改 wire 协议与重连节奏（backoff/tunings 不动）。

## 2. 决策摘要（2026-11-12 复核后全部定稿）

| 决策 | 内容 | 状态 |
|---|---|---|
| 链路相位 | LinkPhase { Connecting, Live, ResyncNeeded, FatalClosed { reason } } 单一字段（Mutex）承载，替换 subscribed_ok / resync_needed / fatal | 定稿 |
| link_health | 由相位派生：Connecting / Live → Up；ResyncNeeded / FatalClosed → Down | 定稿 |
| fatal 恢复 | 不自动重连；FatalClosed 保持可观测（Down + 原因滚动提示），恢复入口 = 用户下次提交 prompt（prompt() 见 FatalClosed 先清 fatal 并重启下行再等待订阅）。不新增键位/可点动作（通知条 API 无可点动作，新增键位超出本单范围） | 定稿（修正） |
| http_ws | 死亡单一载体 PeerState { Alive, Dead(reason), Resetting }（Mutex）+ Notify 唤醒；stop 保留为控制句柄（非状态载体）；reset_mux / ensure_peer 换 peer 时旧 peer 置 Resetting 并清原因 | 定稿 |
| turn_cancel | AgentEnd 与 prompt 流关闭 复位新 token；prompt 起手保留「已取消则换新」补偿 | 定稿 |
| AttachObserve | Ended→Live 仅经显式 resubscribe（restart_downlink）；自动重订阅 不得把 Ended 直接改 Live | 定稿 |
| 历史窗口 | UiModel.history_notice: bool + 按固定文案定位提示行；不再靠 entries[0] 类型判定 | 定稿 |
| Esc 双击 | EscWindow { last_at, suppress_idle_esc } 由 host 单点持有；UiRoot::on_escape(&mut EscWindow) 只读窗口 | 定稿 |

## 3. 链路相位机

Connecting --订阅成功--> Live；Live --ResyncRequired--> ResyncNeeded；ResyncNeeded --rebuild 完成--> Connecting --订阅成功--> Live；Live --断线--> Connecting；Connecting --致命不匹配--> FatalClosed；FatalClosed --用户提交 prompt--> Connecting。

## 4. 测试边界（seam）

- harness/单测：断链自动重连（Connecting→Live）；fatal 后 link_health Down 且 reason 可见；提交 prompt 触发 FatalClosed→Connecting；ResyncNeeded 未消费时 link_health 呈现 Down；Ended→Live 仅在显式 resubscribe 后发生。
- http_ws：死亡原因单一读取；reset_mux 后状态 Alive 且原因清空。
- 历史窗口：提示行前插其它条目后仍能正确替换/移除。
- Esc：双击窗口在 host 单点（同一窗口跨 slot 关闭不误触）。

## 5. 风险与回滚

- 过渡期旧字段（downlink_gen / skip_cold_replay）保留；回滚按 link / peer / turn / observe / history / esc 六片独立。

## 7. 实施前复核（2026-11-12）

| 结论 | 复核 | 证据 |
|---|---|---|
| 链路五字段 + link_health 只看 subscribed_ok | 成立 | remote.rs:128-136；1029-1035 |
| fatal 永久 Down 无恢复 | 成立 | remote.rs:509-515 写入 fatal 后 return；ensure_downlink 在 started=true 后不再重启 |
| resync_needed 需外部消费 | 成立 | remote.rs:390-405 置位；take_resync_rebuild 消费 |
| 连接死亡四载体 | 成立（但 reset_mux 直接摘除 peer，last_err 残留无害；mux() 分读 closed/last_err 存在「已死但无原因」窗口） | http_ws.rs:47-54；362-370；372-382 |
| turn_cancel 无回合结束复位 | 成立 | remote.rs:1136-1142 仅「已取消则换新」；1244-1247 abort 取消 |
| AttachObserve Ended→Live 无条件 | 成立 | remote.rs:540-546 apply_subscribe_result 直接置 Live |
| 历史窗口靠 entries[0] 类型 | 成立 | history_window.rs:81-130 |
| Esc 双击窗口分居两模块 | 成立 | slot_nav.rs:52-67 last_esc_at；input_policy.rs:27-52 suppress_idle_esc |
