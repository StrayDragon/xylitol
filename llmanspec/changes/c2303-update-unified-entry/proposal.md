---
depends_on:
  - c2300-update-cs-capability-split
  - c2301-update-stable-wire-protocol
  - c2290-update-standalone-host
  - c2302-update-host-multi-session
---

# CLI：serve / attach 形状

c2290 产品路径是独立 Host + TUI attach（方案 C）+ 四象限信封。本票不再把「同进程 embed 为默认」当产品位；只钉 `--host/--port`、`serve` 改名、attach URL。方案 A → `c2315-add-loopback-host-tui`。

`--attach` 连已在听的 HTTP host。面本地始终在 TUI。

## Why

host 是端口上的 HTTP 服务器。DSH web 是 `--host` / `--port`（默认 `127.0.0.1:3080`）。本票同一形状，端口避开 3080，用 **18790**。

## What Changes

- embed：同进程经契约自连，不占用 HTTP 绑定。给库用户与 print，不是产品 TUI。
- `xylitol serve --host`（默认 `127.0.0.1`；用户可显式 `0.0.0.0`）`--port`（默认 **18790**；`0` 让 OS 分配并打印实际端口）。关 TUI 不杀它。
- `--attach`：默认 `http://127.0.0.1:18790`；可传 URL 或 `--port`。未在听 → 失败并提示 `serve`。禁止静默改 embed。
- 握手：连上后客户端打开 mux WebSocket，并 unary `subscribe`（session + last_seq）。host 回 `host.describe` / subscribed 帧（协议版本字段）。对不上则断开，不降级。版本不走 WebSocket subprotocol。
- 面本地在 TUI；其余经四象限。多窗写入按 c2300。

## 开放决策

自动 attach、后台拉起、引用计数、`serve --stop`、浏览器 `Origin` / `--trusted-host`（Web 未开闸）、Docker 粗沙盒。

## 非目标

协议闭集、多会话组合根、符合性闸、ACP、方案 A（c2315）、Web UI。

## Further Notes

c2290 已把产品 TUI 默认 attach 做成未在听失败（无静默 InProcess）。本票只收 CLI 形状（`serve` / `--attach`）；握手里的 `subscribe` 与 c2302 journal 对齐。
