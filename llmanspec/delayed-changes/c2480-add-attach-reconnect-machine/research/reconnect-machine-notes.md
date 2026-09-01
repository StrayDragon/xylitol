# 连接重连状态机 外部对照笔记（c2480）

> 调研来源：外部参考实现（生产级 coding agent），2026-08-23 摘录。
> 主文件：其 client 包 `src/solid/connection.ts`、TUI 包 `src/mini/stream-v2.transport.ts`。
> 本笔记仅作选型对照；关键结论已摘要进 proposal。

## 参考实现一手证据

**握手校验**（connection.ts:82-84）：连接后首事件必须是 `server.connected`，
否则视为失败立即重试——把「连上了但不是本服务」与正常连接区分开。

**重连循环**（connection.ts:114-145）：

- attempt 计数退避；单次连接存活 ≥ `reconnectDelay(1s)` 则计数归零
  （防「反复闪断」被误判为持续故障而指数升级）；
- generation 计数使旧 stream loop 的迟到事件自动作废（`current(attempt)` 判定丢弃）；
- managed service 双通道恢复：断线时可整体更换 endpoint/api 实例（版本不匹配换血），
  attempt===1 免延迟立刻重试；
- 连接历史环形缓冲 50 条供 debug 面板。

**事件攒批合帧**（connection.ts:52-61）：publish 攒 10ms flush +
框架 batch()，一串事件合成一次响应式更新。

**UX 宽限期**（其 TUI app.tsx:1245-1269）：初始连接给 5s、重连给 1s 宽限，
超时才显示全屏重连遮罩——注释明言 "Suppress the full-screen overlay
for transient startup and event-stream retry states"（瞬时抖动不打扰）。

**重连后的数据策略**（data.ts:4-5）：不全量刷新——cache 失效标记，
"active UI owners decide what to sync again"（谁挂载谁重验证）。
stream-v2 更细：booting 态先缓冲 live 事件、并行 REST 水合权威投影、
水合完成后按序 flush 缓冲——消除快照与事件流之间的缝隙（transport:1402-1431）。

## 设计动机

- 断线不是异常而是常态事件；状态机的目标是最小打扰 + 快速恢复 + 不串台；
- generation 是比「互斥锁」轻得多的旧循环失效手段。

## xylitol 现状核对（2026-08-23）

- `src/app/core/host_client/http_ws.rs` 共 181 行薄客户端：
  rg 验证无 reconnect/backoff/generation/attempt 相关逻辑；
- WS 握手已有 `ServerFrame::ServerHello { version }`（ws.rs 测试可见），可直接用作首帧校验；
- 断线行为：静默/失败，无分级 UX；无事件合帧（host 侧有渲染节流，client 侧无）；
- 冷恢复已有合约：Host 消息快照一次重建 transcript，禁止 journal 回放当 live 事件——
  重连重建应复用该路径而非发明第二套。

## 落点判断

- 状态机落在 host_client 层；UI 反馈用既有词汇**壳层通告**
  （`push_chrome_toast`，status 上方），不新增信息面概念；
- 重连成功 → 快照投影重建（合约不变）；generation 防新旧循环交错；
- 宽限阈值与合帧窗口做成常量起步，不急着配置化。
