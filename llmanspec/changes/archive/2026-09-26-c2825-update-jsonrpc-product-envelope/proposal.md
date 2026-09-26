---
depends_on: []
branch: sdd/c2825-update-jsonrpc-product-envelope
base_branch: main
base_sha: 7e53fbc3604c9a6b2875b1603fecc32361df2738
---

# 产品信封改为主流 JSON-RPC 运行时（不是字段打补丁）

## Why

产品 TUI ↔ Host 已经是 RPC（method + id + JSON），但外层是自研四象限 tag：通用 JSON-RPC 库不能 parse，横切（租约、幂等、订阅、反向审批）全靠特例字段和第二条 HTTP。把 `RpcMessage` 改名塞进 `params.payload` 只能骗过眼睛，扩展点仍是两套信封。

不计成本：词表不动，**替换自研 RPC 运行时**。ACP / Fory / gRPC-as-TUI **不是**本 change。

证据：`docs/research/agent-interop-surface-2026.md`（ACP 后置）；本 change [research/mainstream-rpc.md](./research/mainstream-rpc.md)。插入点已抽出：`src/protocol/wire/codec.rs`。

## What Changes

- 产品真源为 JSON-RPC 2.0：**单一 RPC 入口**（HTTP 与 WS 同一方法模块），method 只在 JSON，不再以 `POST /api/{method}` 为真源。
- `params` = 该方法的 Command 字段；横切走 header / Extensions / middleware（写者租约、幂等 `id`）。
- 事件流改为 subscription → **notification**（无 id）。握手改为普通方法 result，不再依赖 `ServerHello` 特例帧。
- 审批 / 问卷改为客户端 **unary**（下行 notification 告知）；删除 `POST /api/respond` 假 reverse RPC。
- 业务错误仍是稳定**字符串** `code`（`error.data`）；JSON-RPC 数字码仅载体。未知方法 → `-32601`。
- 词表不变：方法名与 Event 载荷仍是现行 Command/Event 闭集（审批从「禁 unary」改为登记 unary 是方法表行）。
- 迁移：codec 双读 → 翻客户端 → bump `PROTOCOL_VERSION` → 删旧解析（只在 tasks）。
- OpenAPI 仍是信封级调试文档，不是客户端生成真源。

## 非目标

- 不把 TUI 改成 ACP client，不解冻 `c2305`。
- 不引入 Fory / MessagePack；gRPC 不当 TUI 真源（程序脸另票）。
- 不改 Command/Event 变体集合（除审批/问卷的通道从 reverse 改为 unary）。
- 不把 crate 名 / `RpcMessage` 钉进 live spec。

## Capabilities（预估）

- `protocol-app`：r1701 / r1709 / r1696 / r1700 / r1713 / r1704 及信封验收场景。
- `server-core`：通道 + 下行信封（r1778 / r1796 / r1803 / r1804 等）。
- `layer-architecture`：产品路径信封句。
- `app-tui-bridge`：产品 TUI attach 客户端形状。

## Impact

- 字节缝仍经 `protocol::wire::codec`（双读窗）；终态产品路径以 jsonrpsee 模块为准，Salvo 可保留调试 GET。
- BDD 夹具改形状断言（`jsonrpc`、notification、approve unary）；通道纪律从「WS 禁上行」改为「WS 只承载 JSON-RPC」。
- 旧 TUI 二进制在 version bump 后 mismatch 死亡（既有 ath44）。

## Open Questions

- [x] live spec / executable 只写 JSON-RPC **终态**；双读不进 MUST，只在 tasks（2026-09-26）。
- [x] ACP 本轮不做（2026-09-26）。
- [x] 未知方法：单一 RPC 入口下为 JSON-RPC `-32601`（随运行时选择闭合；不再单开 URL 404）。
- [x] RPC 运行时：jsonrpsee 替换自研信封；Salvo 留下 healthz / OpenAPI / 日后 Web 页面与 CORS；浏览器 agent 仍走同一 `RpcModule`，不开 REST 词表（2026-09-26）。
- [x] RPC 挂载路径：`POST /rpc` + `WS /rpc` 为产品入口；`/healthz`、`/docs`、日后 `/` 静态页互不抢（2026-09-26）。
- [x] 写者租约：语义不变（每 session 至多一个写者）；HTTP 请求与响应走 header `X-Writer-Token`；WS 应答用 JSON-RPC 顶层 extra member `writerToken`；`params` / `result` 不携带 token（2026-09-26）。

## Further Notes

- 分层、扩展点、线上形状：[design.md](./design.md)。
- 候选对照：[research/mainstream-rpc.md](./research/mainstream-rpc.md)。
- executable 清单：[research/executable-envelope-bdd.md](./research/executable-envelope-bdd.md)。
- 跨 change 底稿：[docs/research/agent-interop-surface-2026.md](../../../docs/research/agent-interop-surface-2026.md)。
