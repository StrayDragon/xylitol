# c2290 verify 交接（本票必做，勿再假绿）

c2290 把信封、方法表、`HostClient`、产品 TUI attach、specta 闸落地了；**监听器仍是旧 REST**。verify 里这些 WARNING 故意没在 c2290 修——本票才是真 Host。

## 必须接上

- 产品 TUI 已默认 `HttpWsClient` attach `http://127.0.0.1:18790`。本票必须让 `POST /api/{method}` + `GET /api/events.mux` 真响，attach 才能从「未在听失败」变成可会话。
- `sr-env1`（`server-runtime.feature` / `product-path-four-quadrant`）当前只断言客户端实现了 `HostClient`（`InProcessClient` + `HttpWsClient`），**不是**真 bind + POST/WS roundtrip。本票落地后把该场景改成可观察的四象限往返，禁止继续用类型存在冒充 e2e。
- TUI 审批仍走本地 AskHostGateway；`HostClient::respond` 已有座位。本票 ReverseRpc 接上后，审批 MUST 走 `POST /api/respond`。
- `subscribe` unary 未在 `XyRemoteDriver::run()` 调用。journal / last_seq 重连本票做。

## 不要在本票另开 REST

方法表无 `list_sessions` / `session_tree` / `queue_stats`。`XyRemoteDriver` 现对这些返回 unsupported / default。若产品要这些，先改 c2290 方法表（或本票 landing 时扩表并 specta 重生），禁止另开 `/api/v1/...` 产品 REST。

## 明确不在本票

- `InProcessClient` 现为 Echo 座位 → **c2304** 换成与 Host 同表的真 dispatch，禁止 echo 过符合性闸。
- `HostClient` 因 clippy 从 crate 根 re-export；与 embed「不导出 `XyRemoteDriver`」略不一致。卫生可顺手，不是本票阻塞。
