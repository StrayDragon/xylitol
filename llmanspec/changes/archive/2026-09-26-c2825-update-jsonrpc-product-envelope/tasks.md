# Tasks — 产品 JSON-RPC 运行时（探索草稿，待运行时选型确认后定稿）

接缝复用既有：`protocol::wire::codec`、Host HTTP/WS、`HttpWsClient`、`tests/bdd/steps_server.rs`。**不做 ACP。** 终态形状见 [design.md](./design.md)。

## 接缝

| 接缝 | 入口 |
|---|---|
| 字节 codec（双读窗） | `src/protocol/wire/codec.rs` |
| 方法表 | `protocol::wire::REGISTRY` → RpcModule 注册 |
| 横切 | 写者租约 / 幂等 middleware（header + JSON-RPC `id`） |
| 产品客户端 | `HostClient`：`respond`/`mux` 改为 unary + subscribe |
| OpenAPI | 信封级；调试 GET 可留 Salvo |
| 形状 BDD | 见 [research/executable-envelope-bdd.md](./research/executable-envelope-bdd.md) |

## T1 codec 双读（迁移窗）

- [x] `decode` 接受四象限 JSON 与 JSON-RPC 2.0
- [x] 单测：两种字节；非法混杂失败（JSON-RPC 样例 BDD `jsonrpc-envelope-shape`）
- 验证：`protocol::wire::codec` 单测 + `test_pa_env1_jsonrpc_shape`
- [blocked-by: specs landing]

## T2 jsonrpsee 模块挂上（若选型确认）

- [x] `REGISTRY` 驱动 `RpcModule`；Salvo 调试 GET 并行
- [x] 租约 / 幂等走 middleware，不进 `params`
- [x] `session_subscribe` → Event notification；`approve_tool` unary
- 验证：host.describe 200 + 未知方法 `-32601`
- [blocked-by: T1]

## T3 验收场景改口（终态形状）

- [x] protocol-app / server-core executable 按 BDD 清单改口
- [x] 人句 r1700 / r1709 / r1713 / r1704 与 executable 一起改（禁止只改人句）
- 验证：清单 A–D 场景绿
- [blocked-by: T2]

## T4 翻产品客户端并 bump 版本

- [x] 产品 attach 只发 JSON-RPC；删 `POST /api/respond` 依赖
- [x] bump `PROTOCOL_VERSION`；ath44 mismatch 仍致命
- 验证：writer / approval / hello mismatch 真线 BDD
- [blocked-by: T3]

## T5 删旧解析

- [x] codec 不再 decode 四象限 `type` tag
- [x] OpenAPI 信封级；禁止 per-method 第二词表
- 验证：`just qa`
- [blocked-by: T4]
