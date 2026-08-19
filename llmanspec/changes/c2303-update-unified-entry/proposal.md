---
depends_on:
- c2300-update-cs-capability-split
- c2301-update-stable-wire-protocol
- c2290-update-standalone-host
- c2302-update-host-multi-session
branch: sdd/c2303-update-unified-entry
base_sha: 9c2a77fab774ce1514c331920fc14fce90d25b9b
checkpointed: false
---

# CLI：serve / attach 形状

c2290 产品路径是独立 Host + TUI attach（方案 C）+ 四象限信封。本票不再把「同进程 embed 为默认」当产品位；只钉 `--host/--port`、`serve` 改名、attach URL。方案 A → `c2315-add-loopback-host-tui`。

`--attach` 连已在听的 HTTP host。面本地始终在 TUI。

## Why

host 是端口上的 HTTP 服务器。DSH web 是 `--host` / `--port`（默认 `127.0.0.1:3080`）。本票同一形状，端口避开 3080，用 **18790**。

## What Changes

- embed：同进程经契约自连，不占用 HTTP 绑定。给库用户与 print，不是产品 TUI。
- `xylitol serve [--host] [--port]`：默认 `127.0.0.1:18790`；`--host 0.0.0.0` 显式；`--port 0` OS 分配并打印。**无** `server` / `run` 别名。`serve stop` 只打印 SIGTERM；`serve install` 仍 stub。关 TUI 不杀 listener。
- `--attach`：默认 `http://127.0.0.1:18790`；可传 URL 或 `--port`。未在听 → 失败并提示 `serve`。禁止静默改 embed。
- 握手：连上后客户端打开 mux WebSocket，并 unary `subscribe`（session + last_seq）。host 回 `host.describe` / subscribed 帧（协议版本字段）。对不上则断开，不降级。版本不走 WebSocket subprotocol。
- 面本地在 TUI；其余经四象限。多窗写入按 c2300。

## 开放决策

无。本票不包含：自动拉起 Host、引用计数、PID 发现式 `serve --stop`、`--trusted-host` / 非 loopback Origin（Web 未开闸）、Docker 粗沙盒。这些分别见 c2315 / 本文件非目标。

## 非目标

协议闭集、多会话组合根、符合性闸、ACP、方案 A（c2315）、Web UI、自动 attach、后台拉起 Host、浏览器 Origin 白名单。

`serve stop` **只**打印 SIGTERM 指引（c2302 已如此），不发现 PID、不读锁文件。

## Further Notes

- mux Origin：c2302 已允许无 Origin + loopback Origin。非 loopback / `--trusted-host` **不**进本票。
- 握手（mux + `host.describe` + `subscribe`）已在 c2302 `XyRemoteDriver`。本票只把 **CLI 形状**接到可覆盖的 attach URL，不重做握手。
