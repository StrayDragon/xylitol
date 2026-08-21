---
depends_on:
  - c2290-update-standalone-host
  - c2302-update-host-multi-session
---

# 后置：薄 TypeScript 客户端消费 specta bindings

c2290 已把 Rust 类型 SSOT 导出为检入的 `bindings.ts`（无 Web UI）。本票只做 DSH `AbstractApiClient` 形的手写薄层：铸 `rpcId`、`POST /api/<method>`、校验回显、订 mux WS、`POST /api/respond`。Vite SPA / 控制台壳另议，不在本票。

## Why

Web 不能 `use` Rust 类型。没有薄客户端，`bindings.ts` 只是一堆 interface，每个面都会各自发明 fetch/WS 状态机，再次双轨。

## What Changes

- 一个 TS 模块吃 c2290 `bindings.ts`：unary / respond / mux 订阅。方法名与信封字段不得手写第二套。
- **写者租约：** `ClientRequest.writerToken` 由首次非只读 unary 的 `result.value.writerToken` 颁发；同一客户端后续写必须回显。薄层 MUST 把令牌存在客户端实例上（与 Rust `HttpWsClient` 同语义），MUST NOT 每个 fetch 新建匿名客户端再写同一 session。
- 不交付页面、路由、ConversationNode、登录、静态托管。
- 不把 OpenAPI 生成客户端当这条路径。

## Capabilities

- 未来 Web 面的 host 客户端（capability 名落地时再钉；现无 live Web spec）

## Impact

无产品 TUI/CLI 行为变化。给日后 SPA 一个可 import 的协议客户端。

## 非目标

Vite SPA、同源动作 id（delayed `c2325`，不进本票）、salvo oapi、ACP、方案 A。
