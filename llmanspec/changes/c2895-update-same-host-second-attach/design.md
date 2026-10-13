# Design：同 Host 第二路 attach = 一回合一演员，第二路冷观景 + 显式重挂

> 本文件逐条关闭 proposal「重新讨论要点」。产品形态取 **(A) 变体**：第二路不跟看直播、显式 resume 收看；不引入角色系统，占用 = 回合级。

## D1 产品形态：A'（强制再 resume，收窄版）

第二路 attach 落在回合中（`run_inflight`）时：**不重放 tape、不收流式 delta**，只有冷快照（既有 `get_messages`）+ 生命周期/队列/审批帧 + AgentEnd 结束信号；屏幕提示「另一端正在对话，结束后重挂收看」。回合外（含 A 挂机空闲）attach 一律正常：完整订阅 + 可写。先到端在 B 回合期间的既有订阅**继续收直播**（订阅即收看，r1792 不动）——不对称是刻意的：先到者已持有直播带，后来者显式索取。

否决 (B)：常驻观察者角色需要角色协商与升格协议，状态面大；否决 (C)：双打字机即原始 bug。

## D2 占用定义：回合级（`run_inflight`），订阅不占用

- 仅 `run_inflight=true` 算「占用中」。订阅存在但空闲 ≠ 占用：A 挂机时 B 可正常 attach 并成为主动端。
- c2893 linger 语义不变：A 断开宽限内 B resume = 续占（r1793 既有场景覆盖）。

## D3 B 在 A 回合中再问：根提交稳定拒绝，改向共享

- **根提交（prompt）**：回合成中任何连接（含先到端）再发根提交 → 稳定业务错误 `session_busy`（HTTP 200 信封内错误，对齐插话续跑文「忙碌时二次根提交有明确拒绝」）。实现 = 准入处 CAS `run_inflight`，同时关掉「第二 `d.run()` 重叠流」气味。单 TUI 不回归：TUI 忙碌时 Enter 已映射为插话，不发根提交。
- **插话/续跑（steer/follow_up）**：会话级道，任何连接可入队（队列徽标经既有 QueueUpdate 广播可见）。不做按连接的队列隔离。

## D4 B 屏幕内容（被挡时）

冷快照（transcript 条目，r1810 既有；未 AgentEnd 的半截助手行不入快照）+ 固定区提示条（busy → 已结束可重挂两态，词随 `TUI信息呈现与固定区词汇`）+ AgentEnd 通知。非 tape 帧照收：队列深度、审批/问卷、bash 输出。B 的根提交被拒时展示 `session_busy` 错误。

## D5 r1514 豁免句收法

删除「尚未提供多客户端 attach 时，单进程默认路径 须视为已满足」豁免；r1514 改写为真实第二 attach 合约（写者唯一 + 并发根提交稳定拒绝 + 回合中第二 attach 冷观景）。行为细则落 `server-core` 新 requirement（CAS 守卫、观景态订阅语义）；端侧提示落 `app-tui-host` 新 requirement。

## D6 端形态边界：一套规则，无例外

同进程双 TUI、同 TUI 多 tab（c2880，各 tab 独立订阅）、in-process Print/嵌入、HTTP 第二客户端：同一合约。tab 不允许跟看例外；in-process 客户端经既有 broadcast 下行，同等受 CAS 与观景态约束。

## D7 实现气味处置

1. **重放广播全体** → 定向化：journal 重放只发本次新绑定的 mux 订阅者（per-sub tx 直投），在订者不再吃重放突发；in-process broadcast 路径保持现状（仅嵌户道，无第二订阅者重叠问题）。`session/subscribed` 确认帧维持广播（skip_cold_replay 清除语义不变）。
2. **第二 prompt 重叠 `d.run()`** → D3 CAS 准入守卫关闭。
3. **`skip_cold_replay` 端侧过滤** → 保留（冷连防重放，与 D4 服务端观景态互补：观景态下服务端本就不重放 tape，端侧过滤继续防 last_seq=0 直播重放）。

## 观景态判定与退出

- 进入：`bind_mux_to_session` 时 `run_inflight=true` → 该批新订阅者置 observe 态。
- 观察态帧集 = 非 tape 全集（队列/审批/问卷/bash/resources/resync/subscribed/AgentEnd 等生命周期）。
- 退出：仅显式重挂（同会话再次 subscribe；客户端 resume 动作 = 对当前会话重入既有 SwitchSession 重订阅路径）。重挂时若仍 inflight → 维持 observe；空闲 → 完整订阅。host 不自动升格。

## 线协议增量

`SubscribeResult` / `Subscribed` 尾部追加 `inflight: bool`（r1910 尾部追加纪律；旧端按 r1907 忽略）。fbs 真源 + `just codegen-wire` 再生。

## 非目标

跨 Host 抢锁、gpui 第二面、tmux 结对、按连接队列隔离、自动升格写者、占用动画/宽限变更（c2893 不动）。
