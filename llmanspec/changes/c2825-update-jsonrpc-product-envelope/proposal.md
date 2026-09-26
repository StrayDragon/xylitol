---
depends_on: []
---

# 产品信封改为 JSON-RPC 2.0 形状（通道纪律不变）

## Why

产品 TUI ↔ Host 已经是 RPC（method + 相关 id + JSON），但外层是自研四象限 tag。
通用 JSON-RPC 库不能 parse；调试手感却已经是文本 JSON。要把第一方面接到业界信封，
只换外层字段，不换 Command/Event 词表，也不放开全双工 WS。

ACP / Fory / gRPC **不是**本 change：外人面 ACP 保持搁置（`c2305`）；二进制 codec 不做。

证据：`docs/research/agent-interop-surface-2026.md`（2026-09-26 锁定）。
插入点已抽出：`src/protocol/wire/codec.rs`。

## What Changes

- 产品信封改为 JSON-RPC 2.0 对象：`jsonrpc: "2.0"` + `id`/`method`/`params` 或 `result`/`error`。
- **通道不变**：unary / respond 仍 HTTP POST；mux WebSocket **只下行**；审批问卷仍 POST `/api/respond`。
- 词表不变：方法名与 Event 载荷仍是现行 Command/Event 闭集。
- 业务错误仍是稳定**字符串** `code`（放在 JSON-RPC `error.data`）；信封层可用 JSON-RPC 数字码作载体，**不得**让数字码成为产品错误模型。
- `writerToken` 进 `params`（或缺省省略）；`ServerHello` 变为无 `id` 的 notification。
- 迁移：codec **双读**（旧四象限 + 新 JSON-RPC）→ 翻 `HttpWsClient` → bump `PROTOCOL_VERSION` → 删旧解析。
- OpenAPI 调试文档改描述 JSON-RPC 信封；仍不是客户端生成真源。

## 非目标

- 不把 TUI 改成 ACP client，不解冻 `c2305`。
- 不引入 Fory / MessagePack / gRPC。
- 不放开 mux 业务上行。
- 不改 Command/Event 变体集合。

## Capabilities（预估）

- `protocol-app`：r1701 / r1709 / r1696 及信封验收场景。
- `server-core`：r1778 / r1796 / r1803 / r1804 通道 + 下行信封。
- `layer-architecture`：产品路径信封句。
- `app-tui-bridge`：产品 TUI attach 客户端形状。

## Impact

- `protocol::wire::codec` 是唯一字节缝；Host HTTP/WS 与 `HttpWsClient` 只经此模块。
- BDD 夹具已走 codec 编码器；须改形状断言（含 `jsonrpc`、无顶层 `type: client-request`）。
- 旧 TUI 二进制在 version bump 后 hello mismatch 死亡（既有 ath44）。

## Further Notes

- 字段映射与错误模型见 [design.md](./design.md)。
- 跨 change 底稿：[docs/research/agent-interop-surface-2026.md](../../../docs/research/agent-interop-surface-2026.md)。
