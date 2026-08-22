---
depends_on:
- c2290-update-standalone-host
- c2302-update-host-multi-session
branch: sdd/c2320-add-salvo-oapi-docs
base_sha: 637fc6eb3888bbae78c21d169b65cb056d9be41c
checkpointed: true
checkpoint_sha: 637fc6eb3888bbae78c21d169b65cb056d9be41c
---

# unary 调试文档：GET /openapi.json

给 Host 监听器挂 OpenAPI 3.1 调试文档（`GET /openapi.json`），方便 curl / 外部脚本看 unary 面。**不是**类型 SSOT，也不是 TUI/Web 客户端真源。

## Why

specta 盖信封 + WS 下行；人手调试 unary 仍想要 `/openapi.json`。把 oapi 塞进 c2302 会诱使「OpenAPI 当契约」。单独成票，避免与 specta 闸抢 SSOT。

## What Changes

- **server-core** 新增 requirement `sr-oapi1`（unary 调试文档）：
  - Host MUST 在监听器暴露 `GET /openapi.json`，返回 OpenAPI 3.1 文档。
  - 文档 MUST 从已登记 unary 方法表生成：每个登记方法一个 `/api/<method>` 条目（共享信封级 schema）；另含 `/healthz` 与 `/api/respond`。MUST NOT 手写第二套 schema 词表。
  - WS 下行 MUST NOT 作为 OpenAPI path 呈现——以文档说明指向 specta `bindings.ts` 的 `ServerRequest` / `DOWNLINK_METHODS`。
  - 该端点仅调试文档：MUST NOT 作为客户端生成真源（pa-bind1「OpenAPI 非类型 SSOT」不变）。
- **实现**：`src/app/server/` 新增文档构建模块 + 路由挂载（`server` feature 内）。
- **Scalar 调试 UI**：`GET /docs` 托管 Scalar UI（salvo-oapi `scalar` feature，**唯一调试 UI**，指向 `/openapi.json`）；MUST NOT 引入第二套调试 UI。随此把 salvo 升到最新 0.95.2 与 salvo-oapi 同版对齐。
- **BDD**：`server-runtime.feature` 增可执行场景 `@req:sr-oapi1`（走既有 ServerTest harness）。

## 已拍板决策

- **载体**：OpenAPI 文档本体仍由 serde_json 从方法表手构（零逻辑依赖、无 schema 词表漂移）；salvo-oapi 仅作 Scalar UI 托管载体，不参与 spec 构建。偏离本草案早期「salvo oapi」字样；合约本质是「OpenAPI 3.1 调试文档」，与构建方式无关。理由见 design.md D1/D6。
- **路径形态**：逐方法显式条目（`/api/prompt`、`/api/subscribe`…），不做 `{method}` 通配枚举——浏览友好且仍从方法表程序化生成。

## Capabilities

- `server-core`（sr-oapi1）

protocol-app 不动：pa-bind1（TS 真源闸）与 pa-map1（未登记方法失败）语义不变。

## Impact

仅调试面。产品 TUI / 符合性 / specta 闸不变。Host 监听器多一个只读 GET 端点。

## 非目标

AsyncAPI、rspc、swagger-ui 托管页、把 oapi 当第二套词表、Web UI、openapi-typescript / progenitor 客户端生成。

## Further Notes

决策依据与对拍锚点见 `design.md`。
