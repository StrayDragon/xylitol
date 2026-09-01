# Tasks — c2460 Unary Command 幂等准入

- [ ] 1. Host 准入账本：每 session 槽有界 FIFO（128）+ (rpcId → {method, payload, 状态, 结果, 完成通知})；
  语义四件套：首次执行 / 重复回放 / 处理中等待回放 / 同键异载 `idempotency_conflict`。
  单测覆盖四件套 + 超限逐出。
- [ ] 2. `src/app/server/host.rs` 接线：方法表分发前（parse 后 dispatch 前）过账本准入；
  冲错走 `RpcResult::error("idempotency_conflict", …)`（HTTP 仍 200）。
- [ ] 3. `RpcMessage::ClientRequest.rpc_id` 文档注释补幂等语义（重试 MUST 复用同一 rpcId）。
- [ ] 4. 客户端 keyed unary：`HostClient` 新增可携 `rpc_id` 的入口；`http_ws` 实现
  （复用回显校验与 writer_token 捕获）；既有 `unary` 调用点零改动。
- [ ] 5. BDD：`server-core.feature` 已落 3 条 `@executable`（sr-idem 族），实现对应 bindings 并转绿。
- [ ] 6. 门禁：`just fmt` / `just lint` / `just test`（BDD 套件含既有 otel 抖动问题，
  与本票无关则记录不扩大）。
