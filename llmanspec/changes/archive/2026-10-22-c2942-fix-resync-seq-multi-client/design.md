# 设计：修复 resync 回放、transient 模型同步与多客户端订阅绑定

## 1. 决策总览

三处缺陷同源于「订阅绑定不知道发起连接是谁」与「transient 通知复用 journal seq」：

- **绑定定向**：`subscribe` 的执行连接身份经 `OccupiedHost::dispatch_text(body, mux)` 从 `serve_mux` 一路传入 `HostState::bind_mux_to_session`；WS 传 `Some(tx)`（该连接自己的 `MuxSink`），POST `/rpc` 传 `None`。
- **重订回放**：绑定目标从「本次新绑定集合」改为「本次发起订阅的连接（新建或已存在）」；`replay_or_resync` 的定向语义（r2896：他端绑定不引发重放突发）不变。
- **transient 模型同步独立方法**：新增下行 `session/model`（`{session_id, provider, model_id}`），无 `seq`、不写 journal；`sync_model_downlink` 改发该方法。`session/event` 的 `ModelSelect` 事件路径保留（会话历史内模型变更仍可经该形状下行）。
- **上行 notification 语义显式化**：live spec 写明上行 notification 不执行、不应答（与 c2940 design §2.2 一致）。
- **WS 入站错误收口**：`serve_mux` 入站流 `Err` 时不再静默断链，显式发送 close 1009。

## 2. 连接身份的传递链

```text
WS:  serve_mux ─ register_unbound_mux(tx) -> mux_id ─ dispatch_text(body, Some(mux_id)) ─ envelope::dispatch_text ─ dispatch_request ─ handle_unary ─ bind_mux_to_session(slot, last_seq, Some(mux_id))
POST: rpc handler ─ host.dispatch_text(body, None) ─ （同上，None）
```

- `OccupiedHost::register_unbound_mux` 返回连接 id（`u64`）；`OccupiedHost::dispatch_text` 签名：`async fn dispatch_text(self: Arc<Self>, body: &[u8], mux_id: Option<u64>) -> Reply`。
- `serve_mux` 把 `tx` 交给 `register_unbound_mux` 后只保留返回的 `mux_id`，**不再额外持有 `tx` clone**；否则订阅者被移除时 channel 不会关闭，客户端无法感知断线（这会破坏 ath42/ath43/ath44 的 link_health Down 语义）。
- `unbound_mux` 为 `Vec<(u64, MuxSink)>`；`next_mux_id` 单调分配。连接 id 同时用作该 slot 的 subscriber id，重订时无需再做 channel 比较。

### 2.1 `bind_mux_to_session(slot, last_seq, mux_id)`

- `Some(id)`：从 `unbound_mux` 中按 id 取出该连接的 `tx`（找不到说明已绑定）；若 `slot.subscribers` 尚无该 id，则以同一 id 插入 `tx`；`bound` 只含这一个 id。同 WS 重订命中既有 subscriber id，`bound` 非空，`last_seq+1..` 的回放送达该连接。
- `None`（POST 兼容路径）：保持现状——`mem::take` 全部未绑定 mux，并以各自 id 插入 subscriber。
- 之后 `observe` / `replay_or_resync(&bound, …)` / `sync_model_downlink` 维持原调用序。

### 2.2 `session/model`（下行 notification）

```json
{ "jsonrpc": "2.0", "method": "session/model", "params": { "session_id": "s0", "provider": "openai", "model_id": "gpt-5" } }
```

- `xylitol-protocol`：`method::SESSION_MODEL = "session/model"` 进 `DOWNLINK_METHODS`；`SessionModelPayload { session_id, provider, model_id }`（serde，`lib.rs` 再导出）。
- `xylitol-serve`：`sync_model_downlink` 广播 `downlink_notification(SESSION_MODEL, …)`，删去 `journal.max_seq()` 读取；不追加 journal、不消耗 seq。`writer_model` 查找保持。
- `xylitol-app-tui`：`Downlink::SessionModel(SessionModelPayload)` 解码分支；`remote.rs` 将其投影为 `XyEvent::ModelSelect { provider, model_id }`（同旧 ModelSelect 路径：尽力解析 `ModelInfo` 更新缓存后推入事件流）。旧 `session/event` ModelSelect 投影路径保留作兼容。
- 方法表其余语义不动：`session/model` 只进下行方法清单，不新增 unary。

## 3. 观景态与定向回放的交互

- 观景准入（r2895）不变：`inflight && last_seq == 0` 才进入 observing，不重放 tape。
- 定向回放（r2896）不变：他端绑定不给在订者发重放；本变更只是把「发起连接」从「新绑定者」扩为「新绑定者或同 id 重订者」。
- `session/model` 经 `broadcast` 发全体订阅者（含观景端）——模型徽标是固定区收敛信号，不是 tape；旧实现经 `session/event` 时观景端会被 `observe_passes` 过滤掉，属附带修复。

## 4. WS 入站错误收口（1009）

- `serve_mux` 的 `recv_loop` 遇 `inbound.next() == Err(_)` 时置 `inbound_error` 标志后退出；`tokio::select!` 结束后若置位（且未因 binary 已发 1003），尽力发送 `Message::close_with(1009, "message too big")` 并 `close()`。
- 不区分 capacity 与其他入站错误：连接终止路径一致，统一 1009（Upgrade 已设 `max_message_size(MAX_BODY_BYTES)`，超限帧是最常见成因）。
- 该分支只发关闭帧，不改变 `tx` 生命周期；连接关闭仍由 `tx` 被移除后 channel 关闭驱动。

## 5. 测试策略

- serve 单测（`listen.rs` / `host.rs`）：双 unbound mux 定向绑定、同 WS 重订回放、双客户端跨槽隔离、`session/model` 形状（method/params、无 seq、journal 不递增）、入站错误 1009。
- protocol 单测：`DOWNLINK_METHODS` 含 `session/model`；payload serde 往返。
- app-tui 单测：`decode_downlink` 识别 `session/model`；remote 投影行为沿用既有 harness。
- BDD（`steps_server.rs`）：`订阅者收到模型同步事件且模型为 fake-model` 与 `journal 序号未因绑定递增且两次均收到模型同步事件` 改期望 `session/model`（params `provider`/`model_id`）；场景名与步骤文本不变。

## 6. 风险与边界

- `OccupiedHost` 签名变更波及 `listen.rs` 内 FakeHost 与 BDD 直调 `bind_mux_to_session` 的步骤（补 `None` 参数）；kernel `occupancy::runtime` 只经 trait 对象使用，不受影响。
- POST `/rpc` subscribe 保持「绑定全部未绑定连接」的既有行为（legacy 兼容），spec 明示；产品 TUI 的 `HttpWsClient.unary` 实际走同一 WS，天然享受定向绑定。
- 不处理 P3 清理；不动租约、幂等、方法表其余语义。
