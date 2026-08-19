---
depends_on:
  - c2300-update-cs-capability-split
  - c2301-update-stable-wire-protocol
  - c2290-update-standalone-host
  - c2302-update-host-multi-session
---

# 产品位：embed 默认 + 显式 attach

> **默认拓扑已改。** c2290 产品路径是独立 Host + TUI attach（方案 C）。本票不再把「同进程 embed 为默认」当产品位；`--host/--port`、`serve` 改名、方案 A（一条命令 loopback）另议。

`--attach` 连已在听的 HTTP host。面本地始终在 TUI。

## Why

host 是端口上的 HTTP 服务器。DSH web 是 `--host` / `--port`（默认 `127.0.0.1:3080`，拒绝 `0.0.0.0`）。本票同一形状，端口避开 3080。

## What Changes

- embed：同进程经契约自连，不占用 HTTP 绑定。退出即该进程内 host 没了。
- `xylitol serve --host`（默认 `127.0.0.1`；用户可显式 `0.0.0.0`，自己负责暴露面）`--port`（默认 **18790**；`0` 让 OS 分配并打印实际端口）。关 TUI 不杀它。本票不做 Docker 粗沙盒。
- `--attach`：默认 `http://127.0.0.1:18790`；可传 URL 或 `--port`。未在听 → 失败并提示 `serve`。禁止静默改 embed。
- 握手：连上后客户端 `Subscribe { session_id, last_seq }`，host 回 `ServerHello { protocol: 1 }` + `Ack`。`protocol` 对不上则断开，不降级。版本只走这一字段，不用 WebSocket subprotocol。
- 面本地在 TUI；其余经契约。多窗写入按 c2300。

## 开放决策

自动 attach、后台拉起、引用计数、`serve --stop`、浏览器 `Origin` / `--trusted-host`（Web 未开闸）、Docker 粗沙盒。

## 非目标

协议闭集、多会话组合根、符合性闸、ACP。
