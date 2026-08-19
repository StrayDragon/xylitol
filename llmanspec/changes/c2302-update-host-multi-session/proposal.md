---
depends_on:
  - c2300-update-cs-capability-split
  - c2301-update-stable-wire-protocol
---

# host 多会话与 salvo 契约传输面

N 客户端 ↔ 1 个 HTTP 监听器。产品语义只走 WebSocket 上的 Command/Event（c2301）。栈：salvo 0.94（`salvo-websocket` / `salvo-realtime` / `salvo-graceful-shutdown` / `salvo-testing`）。REST 不再承载产品动词。

## Why

现状一把 `Mutex` + 锁文件 + port+1 + REST 产品面。要的是可连上的监听器，绑定占用即失败，多 session 各有 journal 与写者。

## What Changes

- 监听 `127.0.0.1:18790`（`ServerConfig` / 现有 `server run --port`）。`--host` 可显式 `0.0.0.0`（本票配置字段即可；CLI 旗标形状归 c2303）。占用同一 `{addr,port}` → EADDRINUSE，不 port+1，无整机锁文件。
- 多 session：每 session 独立 journal / seq / 写者。一写者；第二连接只读，写入回产品错误。
- 资源按 **本监听器进程** 装配。reload 作用于该进程、所有连接共享。MCP 池 key 仍为意向，本票不落地新 key。
- 传输：salvo `WebSocketUpgrade` 单端点升级；`Subscribe` 选 session。健康检查可留 HTTP，但 MUST NOT 再提供 run/abort/tree 等产品 REST。
- RemoteDriver 改走同一套 WS 帧，不再 POST 产品 REST。
- 优雅停机：`ServerHandle::stop_graceful`；停则连上的客户端断开。
- 保留现有 `server run/install/stop` 动词（c2303 再改 `serve`/`--attach`）。`stop` 不再靠锁文件。

## 非目标

进线 CLI（`--attach` / `serve` 改名）、符合性闸、ACP、二进制编码、UDS 默认发现、Docker 粗沙盒、MCP 池 key 落地、embed 的 in-process carrier 实现（仍走现有 InProcessDriver + 同一 dispatcher 语义）。
