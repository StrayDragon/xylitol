# 写者租约生命周期（c2893 根因）

日期：2026-10-11。只读代码 + 实机 hang 日志对照；不是 live spec。

## 结论

防双写（r1793 session 写者租约）**只 mint、不释放**。WS 连接断开或用户离开 session 时，槽上 `writer_token` 仍在。下一客户端（同机再 attach、切回该 session）presented 对不上 → `writer_conflict`。

Working 空转不是 mermaid，也不是「只差把 Error 当成 AgentEnd」。是租约幽灵 + RemoteDriver 把 **业务错误当成功**：`run()` 里 `host.unary("prompt")` 只认传输失败，`RpcResult { ok: false, writer_conflict }` 走 Ok 分支，然后永远等 AgentEnd。

打补丁（Error 收口 / spawn 里塞一条 Error）能让这一次看见红字，幽灵牌还在，后人再踩。根因是：写者身份必须跟「谁占用这条 session」同生共死。

## 因果链（实机 `c6a98522-…`）

1. 产品 TUI attach 本机 `xylitol serve`（`DEFAULT_ATTACH_URL`，未在听失败）。Host 进程比 TUI 活得长。
2. 第一轮 prompt：`take_writer_lease` mint，token 进槽 + 进客户端 `HttpWsClient.writer_token`（注释：`one TUI = one writer lease`）。
3. TUI 退出：`XyRemoteDriver::Drop` 只 `session_life.cancel()`。服务端 `handle_mux` 在 close / stream 结束时 **直接 return**，没有清 `SessionSlot.writer_token`。
4. 再 `--session` 同一槽：新进程 token=`None`。槽上仍是旧牌。
5. `begin_run` 立刻 Working。`driver.run()` 构造 stream（`host_run_await 0ms`）。stream 内 `host.unary("prompt")` 返回 `Ok(RpcResult { ok: false, code: writer_conflict })`——对 `Result<RpcResult, HostClientError>` 来说这是 **Ok**。Host **没有** spawn `InProcessDriver::run`（所以没有第二轮 `run_ensure_session`）。
6. stream 进入 `loop { wait AgentEnd }`。mux 不会有本轮事件。spinner 不拆。
7. Esc `ClearQueue`：仍无匹配 token → 同一 `writer_conflict`。

`self.unary()`（带 `into_std()`）会把业务错误变成 `Err`；`run()` **没用它**，直接 `self.host.unary`。

## 代码锚点

| 处 | 路径 | 行为 |
|---|---|---|
| mint | `src/app/server/host.rs` `take_writer_lease` | `None`→发牌；presented 匹配→续；否则 `writer_conflict`。无 release API。 |
| 守卫 | 同文件 `WriterLease` | 无 `Drop`。stamp 应答，不清槽。 |
| WS 占用 | `src/app/server/http.rs` `handle_mux` | 连接本地 `writer: Option<String>`。`is_close` / recv 结束 / send 失败 → 退出，**不碰槽上 token**。 |
| 客户端 | `src/app/core/host_client/http_ws.rs` | 一把 token 跨 clone；进程没了就没了。 |
| 切会话 | `XyRemoteDriver` `SwitchSession` | `reset_mux` 换下行；**不** release 旧槽。`switch_session` 本身是 `Auth::Writer`。 |
| 收口误判 | `src/app/core/driver/remote.rs` `run` | `host.unary("prompt")` 只 `Err` 才 yield Error；业务 `ok: false` 当成功。`is_end` 只认 `AgentEnd`。 |
| 合约缺口 | `server-core` r1793 | 只规定至多一把、HTTP 不是身份、缺/错拒。**未规定**断连 / 离槽释放。现有场景只测「B 无 token 冲突」，不测「A 走了 B 能拿」。 |

## 不是根因（可作第二层，不能当本票主修复）

- spawn 里 `driver` 为 `None` 直接 `return`：acquire 已 `materialize_writer_at`，本 hang 未走到 spawn。
- bridge 非 abort Error 不 Idle：本 hang 的 Error **根本没 yield**。修好 unary 观察之后才看得到；Idle 是可见性，不是锁。
- mermaid-text / `begin_run` 本身：只解释「为什么先看到 Working」。

## 根因修复形状（尚未拍板）

写者身份 = **当前占用该 session 槽的那条 WS 连接**。

- 该连接 close → 若槽上仍是它 mint/续的那张牌，MUST 清牌。
- 同一连接切走该 session（SwitchSession / 再 subscribe 别的 id）→ 旧槽 MUST 清牌（客户端只有一把 token，旧牌客户端已经拿不住）。
- HTTP POST 仍按 r1793：每次新 TCP，token 只在 header，连接不是身份；幽灵只发生在 WS 长连接。
- 不偷别人的牌（last-writer-wins 会废防双写）。
- 不靠客户端 Drop 发 release：`kill -9` 仍泄漏。释放 MUST 在 Host 观察连接/占用结束时做。

`run()` 观察 `RpcResult.ok`：prompt 未准入 MUST 结束这条 stream 并露出错误。这不是「把 Error 当成 AgentEnd 的补丁」，是「未开跑就不得进入等 AgentEnd」。没有这一层，租约再漏一次仍是永久 Working。

## 明确不做（本调研）

- 多客户端同时写同一 session。
- TTL 过期（窗口内仍幽灵）。
- 只改 TUI 文案 / 只 Idle spinner。
