---
depends_on:
  - c2290-update-standalone-host
  - c2302-update-host-multi-session
---

# 后置：salvo oapi 仅 unary 调试文档

给 `POST /api/{method}` 与 `/api/respond` 挂 OpenAPI 3.1（salvo `oapi`），方便 curl / 外部脚本看 unary。 **不是** 类型 SSOT，也不是 TUI/Web 客户端真源。

## Why

specta 盖信封 + WS 下行；人手调试 unary 仍想要 `/openapi.json`。把 oapi 塞进 c2302 会诱使「OpenAPI 当契约」。单独后置，避免与 specta 闸抢 SSOT。

## What Changes

- 可选 feature 或调试路由：unary 的 OpenAPI。方法名 / payload **必须**从 c2290 方法表来，禁止另写 schema。
- WS 下行 **不** 进 OpenAPI（没有一等 server-push 模型）。文档里用一句话指向 specta `bindings.ts` 的 `ServerRequest`。
- MUST NOT 用 openapi-typescript / progenitor 生成产品客户端。

## Impact

仅调试面。产品 TUI / 符合性 / specta 闸不变。

## 非目标

AsyncAPI、rspc、把 oapi 当第二套词表、Web UI。
