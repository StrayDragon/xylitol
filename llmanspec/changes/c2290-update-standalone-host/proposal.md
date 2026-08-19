---
depends_on: []
blocks:
  - c2302-update-host-multi-session
  - c2303-update-unified-entry
---

# 独立 Host：产品 TUI 只当 RPC 客户端

产品默认改为方案 C：先有独立 Host 进程，TUI 与其它面一样经 Req/Resp 连接它。方案 A（同进程 loopback、一条命令起 Host+TUI、孤儿 Host）只记在 `design.md`，本票不落地。

## Why

TUI 若继续走 `InProcessDriver` 直握 runtime，其它 client 跨进程进来就会有两套入口、两套写者。自研 Command/Event 焊在全双工 WS 上，后面每加一个面都要会那套帧。要先把 Host 做成可插多种投递的 RPC 服务，TUI 只当普通客户端；体验仍是流式 token / Esc 中止 / 审批，只是编程模型改成 request、notification、反向 request。

c2300 钉的是角色（client ≠ host）。本票改**产品默认拓扑**：产品 TUI 必须连已在听的 Host。库嵌入（`xylitol::embed`）与 print 仍可同进程，不当产品 TUI 默认。

## What Changes

- 产品 TUI：RPC 客户端。默认连 `http://127.0.0.1:18790`（可 `--attach`）。未在听 → 失败并提示先 `server run`（或日后 `serve`）。MUST NOT 默认 InProcess，MUST NOT 静默改 embed。
- Host：独立监听器进程；会话 / 写者 / 待审批挂在 Host 上，连接只是投递。
- 信封：JSON-RPC 2.0 风格 `id` / `method` / `params`（unary HTTP POST）+ 下行通知流（SSE）。反向审批 = Host 在下行发 request，client 再 POST 应答。方法表可与现行 Command 名对应。MUST NOT 以全双工 WS 上的自研 Command/Event 作为产品协议。
- `server` 进 default features（同一二进制既能 `server run` 又能 `tui --attach`）。
- c2302 暂停 apply：其「产品语义只走 WS Command/Event」作废，待本票归档后按本信封重写。c2303 的「embed 默认」让位给本票；A 落地时再开票。

## Capabilities

- `layer-architecture`：产品 TUI 要求监听器；角色仍 ≠ 监听器定义
- `app-tui`：废止默认 InProcess
- `cli-entry`：attach / 未在听失败
- `protocol-app`：Req/Resp + 通知为产品真源
- `server-core`：HTTP RPC + 下行通知；会话在 Host

## Impact

产品启动从「一条 `xylitol` 即跑」变为「先 Host、再 TUI」。print 与库嵌入不变。ACP 不在本票。UDP 等其它投递只要求能载同一信封，本票不实现。

## 非目标

方案 A 与孤儿 Host、c2303 的 `serve` 改名、salvo 换栈、符合性闸、ACP、Docker 粗沙盒、Web Origin。
