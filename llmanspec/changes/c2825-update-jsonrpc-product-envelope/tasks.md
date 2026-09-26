# Tasks — 产品信封 JSON-RPC 2.0 形状

接缝复用既有：`protocol::wire::codec`、`HttpWsClient`、`tests/bdd/steps_server.rs` `encode_client_request`、protocol-app / server-core BDD。不新开 harness。**不做 ACP。**

## 接缝

| 接缝 | 入口 |
|---|---|
| 字节 codec | `src/protocol/wire/codec.rs` |
| HTTP unary/respond | `src/app/server/http.rs` `parse_envelope` |
| mux 下行 | `http.rs` hello + send_loop |
| 产品客户端 | `src/app/core/host_client/http_ws.rs` |
| OpenAPI | `src/app/server/oapi.rs` |
| 形状 BDD | `four-quadrant-envelope-shape` → JSON-RPC 形状；`unary-stable-error-envelope` |
| 行为 BDD | writer lease / reverse RPC / product-path-four-quadrant（通道断言保留） |

## T1 codec 双读

- [ ] `decode` 接受四象限 JSON 与 JSON-RPC 2.0，产出同一 `RpcMessage`
- [ ] 单测：两种字节 → 相等 IR；非法混杂失败
- 验证：`protocol::wire::codec` 单测
- [blocked-by: specs landing]

## T2 出站按请求方言

- [ ] unary/respond 应答与 mux 帧：若请求是 JSON-RPC 则 `encode` JSON-RPC，否则仍四象限（双读窗）
- [ ] `writerToken` / hello notification 按 design 表
- 验证：codec 单测 + `unary_host_describe_ok` 仍 200
- [blocked-by: T1]

## T3 验收场景改口（形状，不改通道）

- [ ] protocol-app / server-core 可执行场景：断言 `jsonrpc`、`id` 回显、字符串业务码在 `error.data`
- [ ] **保留** WS 不收业务上行、POST respond、HTTP 200 载体成功
- [ ] `steps_server` 夹具继续走产品 encoder（翻方言自动跟上）
- 验证：`test_pa_env1_*` / `test_ip3_stable_error` / `test_sr_env1_four_quadrant`
- [blocked-by: T2]

## T4 翻产品客户端并 bump 版本

- [ ] `HttpWsClient` 只发 JSON-RPC
- [ ] bump `PROTOCOL_VERSION`；hello 用 notification
- [ ] ath44 mismatch 仍致命
- 验证：真线 attach BDD（writer / approval / hello mismatch）
- [blocked-by: T3]

## T5 删旧解析与调试文档

- [ ] codec 不再 decode 四象限 `type` tag
- [ ] OpenAPI schema 改为 JSON-RPC 信封级；仍禁止 per-method 第二词表
- 验证：`just qa`
- [blocked-by: T4]
