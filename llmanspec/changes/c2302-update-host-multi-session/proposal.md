---
depends_on:
  - c2300-update-cs-capability-split
  - c2301-update-stable-wire-protocol
---

# 产品位：host 多会话 + 契约传输面

N 客户端 ↔ 1 个 HTTP host，host 承载多 session。栈：salvo。REST 不承载产品语义。传输面按 c2301：HTTP 升级 WebSocket 走 Command/Event。

## Why

现状一把 `Mutex` + 一个合成会话。要的是可连上的 HTTP 服务器，不是整机锁。

## What Changes

- `xylitol serve` 听 `127.0.0.1:18790`（避开 DSH 默认 3080 与现状代码 8080）。`--port` 覆盖。停则连它的客户端断开。
- 多 session：生命周期与写入规则按 c2300 / c2301。
- 配置 / 模型注册表 / MCP / 工具按 **这个 host 进程** 装配。MCP 池 key 意向 `(mcp_name, canonical cwd)`。reload 作用于该 host，所有连接共享。
- 同一 `{host,port}` 占用即失败（EADDRINUSE），不自动 +1。另一 `--port` 可再起。

## 开放决策

- MCP 池 key 的落地阈值。
- embed 同进程的 **carrier 实现**（channel vs 套接字）归 c2301/c2303。产品缝已由 c2300 design 钉死：client 与 host 走同一 dispatcher，禁止「只有套接字才进 host」。

## 非目标

进线、符合性闸、ACP、二进制编码、整机互斥锁、UDS 作为默认发现。
