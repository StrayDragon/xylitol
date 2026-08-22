# 设计注记：/openapi.json unary 调试文档

## 代码事实锚点（对拍表）

| 合约点 | 实现锚点 | 测试锚点 |
|---|---|---|
| 方法名唯一来源 | `src/protocol/wire/method.rs` `UNARY_METHODS`（~25 个）| method.rs 单测 |
| 信封形态（文档 schema 的唯一依据）| `src/protocol/wire/envelope.rs` RpcMessage/RpcResult/RpcError | envelope roundtrip |
| 路由挂载点 | `src/app/server/http.rs::router`（healthz / api/respond / api/events.mux / api/{method}）| http.rs TestClient 测试 |
| BDD harness | `tests/bdd/steps_server.rs` `http_status(port, method, path, body)` helper；`ServerTest` fixture | server-runtime.feature 既有场景 |

## 决策记录

1. **D1 载体 = serde_json 手构，不引 salvo-oapi**：
   - 文档内容是「方法表枚举 × 一份信封级 schema」，无逐方法类型差异，宏推导体系（#[endpoint]）用不上主力能力；
   - 零新增依赖符合仓库 lean 哲学；salvo-oapi 上游版本耦合与 gpui 同类风险；
   - 结构正确性由单测守护（解析回读断言 openapi 版本、逐方法条目数、禁含 events.mux）。
   - 偏离草案「salvo oapi」字样已回写 proposal。
2. **D2 逐方法显式条目**：每个 UNARY_METHODS 条目生成 `/api/<method>` POST path（共享 `$ref` 信封 components + operationId=方法名）。比 `{method}` 通配 + enum 更浏览友好；仍为程序化生成，无手写词表。
3. **D3 信封级 schema 粒度**：components 只描述 RpcMessage（client-request/client-response）、RpcResult（ok/value/writerToken）、RpcError（code/details）；payload 字段保持 object 粒度。具体 payload 形状的真源是 specta bindings——文档中 info.description 指向之。**禁止**在 OpenAPI 里展开逐 variant schema（那才是第二套词表）。
4. **D4 WS 下行呈现方式**：不出现在 paths；在 info.description 固定一句指向 bindings.ts `ServerRequest` 与 mux 通道说明。
5. **D5 feature 门**：挂在既有 `server` feature 内（默认开）。调试文档是只读 GET、零运行时成本；独立 opt-in feature 反而让默认产物缺文档、qa 需要额外矩阵。
6. **D6 Scalar UI + salvo 升级（用户拍板）**：`GET /docs` 托管 Scalar 作为**唯一**调试 UI；依赖 `salvo-oapi`（`default-features=false, features=["scalar"]`，仅用其 Handler，不参与 spec 构建）；salvo 0.94 → 0.95.2 与 salvo-oapi 同版对齐（Handler trait 跨版本不兼容，必须同版）。升级后全量编译零破坏。

## 测试边界（前置确认）

- **可执行 BDD**：server-runtime.feature 新场景走既有 ServerTest harness（复用 `http_status` helper），步骤为新字面量步骤（given 服务端启动复用既有步骤）。
- **单测**：oapi 模块内结构断言（openapi 版本 / 方法条目全集 == UNARY_METHODS / 无 events.mux / respond 与 healthz 存在）。

## apply 预期

新模块 ~150 行 + 路由一行挂载 + BDD 接线（bindings_server.rs 场景注册 + steps_server.rs 步骤定义）。零生产行为变更（新增只读端点除外）。
