# Design — c2460 Unary Command 幂等准入

## 现状核对（代码事实）

- 信封 `RpcMessage::ClientRequest.rpc_id`（必带）：`HttpWsClient::unary` 每次调用新造 UUID
  （`src/app/core/host_client/http_ws.rs`），Host 回显且不匹配报 `RpcIdMismatch`——
  已是「调用方产 ID」，只差「服务端唯一性判定」。
- `Command` 每变体带 `id: Option<String>`（"for correlation"），但 host.rs 方法表
  硬编码 `id: None`，该字段在 HTTP 产品路径未被消费。
- 客户端当前**无重试循环**（unary 超时直接报错），重试幂等是 c2480 重连机器的前置。

## 决策

- **D1 键载体：复用信封 `rpcId`**（2026-09-01 拍板）。零 wire 形状变更；
  `Command.id` 保持 correlation 语义不动。未来若需「每次尝试新 rpcId + 稳定幂等键」正交语义，
  再加可选字段（加法变更，便宜）——pre-0.0.1 不预留。
- **D2 准入语义**：键 = (session 槽, rpcId)。
  - 首次准入获胜：首次执行，结果入账；重复提交回放首次结果（含错误结果）。
  - 处理中重复：等待首次完成后回放同一结果（账本项带完成通知，如 oneshot/watch），
    MUST NOT 并行执行——这正是超时重试双发 steer 的主杀场景。
  - 冲突：同 `rpcId` 但 method 或 payload 不同 → `ok=false, code=idempotency_conflict`，
    不执行。HTTP 状态仍 200（ip3：信封合法即载体成功）。
- **D3 账本**：每 session 槽进程内 FIFO 有界（默认 128 条），准入时插入、完成时填结果、
  超限逐出最旧。不持久化：重启即清，跨重启重试视为新命令（proposal 非目标）。
  payload 比较用 `Value` 相等，不引入指纹哈希（无稳定哈希需求，账本有界内存可控）。
- **D4 覆盖范围：全部已登记 unary**。不做读写分法——单机个人工具下有界账本成本可控，
  且避免维护「哪些方法要幂等」的第二张方法清单；只读重复回放无副作用。
- **D5 客户端**：`HostClient` 增加可携 `rpc_id` 的 unary 入口（既有 `unary` 保持新造 UUID 行为，
  调用点零改动；http_ws 单实现，复用既有回显校验 / writer_token 捕获路径）。
  本票不实现自动重试循环与退避（c2480 消费本入口）；driver 侧暂不接线自动重试。
- **D6 specs landing**：并入 `server-core.feature`（2026-09-01 拍板）——新增
  `@req:sr-idem1 @human`（幂等准入合约）+ 3 条 `@executable`
  （重试回放首次结果 / 同键异载冲突 / 处理中等待）。header `scope` 扩为 `src/`（含客户端落点）。

## 关联

- c2480（attach 重连状态机）：其安全重试依赖本票 keyed unary；本票先行为其铺路。
- c2485（凭据门禁）/ c2465（就绪窗口）：与本票无落点交集。
