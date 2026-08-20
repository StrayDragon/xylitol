---
depends_on: []
blocks:
  - c2302-update-host-multi-session
  - c2303-update-unified-entry
  - c2304-add-conformance-gate
  - c2305-update-acp-provider-adapter
  - c2310-add-web-ts-client
  - c2315-add-loopback-host-tui
  - c2320-add-salvo-oapi-docs
---

# 独立 Host：四象限 RPC + 产品 TUI 只当客户端

产品默认改为方案 C：先有独立 Host 进程。TUI / 未来 Web 连同一套**四象限信封**（对齐 DSH apiproxy，不是 JSON-RPC 2.0，也不是全双工 WS 上的 Command/Event）。方案 A（一条命令 loopback）另开草案，不进本票。

> **作废旧 landing。** 本地/远程 `sdd/c2290-update-standalone-host` 曾把 JSON-RPC 2.0 + SSE 写进 live specs——物理层与信封形状都错了，**禁止** `attach` 该分支。新 Specs landing 须在重写后的规划壳上 `change start` 新分支。

## Why

TUI 若继续 `InProcessDriver` 直握 runtime，其它 client 进来就会两套入口、两套写者。自研 Command/Event 焊在全双工 WS 上，每加一个面都要会那套帧。DSH 已落地的做法是：信封与通道解耦；unary 永远 HTTP POST；网络下行是 **WebSocket、且不收业务上行**；进程内可用同一信封的 in-process 客户端。ACP 不走这套信封。

c2300 钉的是角色（client ≠ host）。本票改**产品默认拓扑 + 信封**。库嵌入与 print 仍可同进程，不当产品 TUI 默认。

## What Changes

- 产品 TUI：RPC 客户端。默认连 `http://127.0.0.1:18790`（可覆盖）。未在听 → 失败并提示先起 Host。MUST NOT 默认 InProcess，MUST NOT 静默改 embed。
- Host：独立监听器进程；会话 / 写者 / 待审批 / 观测挂在 Host 上。连接断开 ≠ 会话消失。
- 信封：四象限 discriminated union（`client-request` / `server-response` / `server-request` / `client-response`），`rpcId` 由发起方铸造、应答回显。MUST NOT 以 JSON-RPC 2.0 数字错误码为产品错误模型。MUST NOT 以全双工 WS 上的自研 Command/Event 为产品协议。
- **一份方法表**（见 `research/method-table.md`）：unary 名 + payload + 返回；`Command` 变体即 payload，不另抄 DTO。`Quit` 面本地；`ApproveTool`/`AnswerQuestion` 走 `POST /api/respond`。下行多数生命周期进 `session/event`（payload 仍是 `Event`），不按变体爆炸成方法。
- 本票物理示例（C）：unary + `respond` = `POST /api/<method>`；下行 = WebSocket 文本帧，每帧一个 `ServerRequest`。SSE 只保留给进程内同构测试，**不是**产品 TUI 真源。
- **一个 typed client、两个 carrier**：`InProcessClient`（测试 / 同构）+ `HttpWsClient`（产品 TUI）。TUI MUST 经该 trait；未在听失败。禁止静默 InProcess。print / `xylitol::embed` 仍可直握现有 InProcessDriver（不必先换信封）。
- **类型 SSOT + specta 闸（本票落地，无 Web UI）**：信封与方法 payload `#[derive(specta::Type)]`；CI 导出 `bindings.ts` 并 diff。TUI 同 crate 直接 `use`。OpenAPI / AsyncAPI / salvo oapi **不当 SSOT**（OpenAPI 盖不住 WS 下行）。
- `server` 进 default features。
- c2302 按本方法表展开 POST/WS 载体。c2304 双 client 同表。c2303 不再把 embed 当产品默认。

## Capabilities

- `layer-architecture`：产品 TUI 要求监听器；角色仍 ≠ 监听器定义
- `app-tui`：废止默认 InProcess
- `cli-entry`：attach / 未在听失败
- `protocol-app`：四象限为产品真源；方法表 SSOT；Command/Event 闭集仍是方法/帧载荷
- `server-core`：HTTP POST + WS 下行；会话在 Host
- `test-*`：specta `bindings.ts` 检入 + qa 闸（漂移即红）

## Impact

产品启动从「一条 `xylitol` 即跑」变为「先 Host、再 TUI」。print 与库嵌入不变。ACP 不在本票。观测（fastrace）留在 Host 内核，不随 HTTP 皮重写。

## 非目标

方案 A 与孤儿 Host（另草案）、c2303 的 `serve` 改名、salvo 换栈（c2302）、符合性双跑（c2304）、ACP、Docker 粗沙盒、Web UI / Vite SPA、薄 TS 客户端（消费 bindings 的手写层另草案）、salvo oapi 调试文档。

## Further Notes

- 方法表：[`research/method-table.md`](./research/method-table.md)
- 类型共享：[`research/type-sharing.md`](./research/type-sharing.md)
- DSH 一手：`../deepseek-harness/.agents/notes/implemented/architecture/2026-07-19-gui-layering-and-rpc-protocol.md` 与 `2026-08-04-websocket-downlink-carrier.md`
