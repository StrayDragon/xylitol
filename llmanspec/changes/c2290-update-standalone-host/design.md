# Design：独立 Host + TUI 作 RPC 客户端

## 1. 拓扑（本票交付：C）

```text
xylitol server run     →  Host 听 127.0.0.1:18790
xylitol tui [--attach] →  只当 client，HTTP Req/Resp + 下行通知
xylitol print …        →  仍 InProcess（一次性，无多 client）
库 xylitol::embed      →  仍 InProcess（给 crate 用户，不是产品 TUI）
```

未在听：TUI 非零退出，提示启动 Host。禁止静默落到 InProcess。

会话状态、写者位、待审批、journal 在 Host。连接断开 ≠ 会话消失。TUI 崩溃不杀 Host。

## 2. 信封（投递可换，词表不可双轨）

四类消息，与物理通道解耦：

| 象限 | 谁发起 | 本票物理（C） |
|---|---|---|
| Client request / response | client | `POST` JSON-RPC 2.0（`id, method, params` → `id, result\|error`） |
| Notification（token / 工具更新） | host | SSE（或日后任意下行流），每帧一个 notification |
| Server request / client response | host 问审批 | 下行 `id` 请求；client `POST` 应答同一 `id` |

方法名可对现行 `Command` / `Event` 变体，但线信封 MUST 是上述四象限，MUST NOT 把「这条 WebSocket 上的 serde 枚举」当成产品协议。

WS / UDS / UDP：只允许作为**同一信封**的另一种投递。本票不实现。禁止为本机再发明第二套词表。

现行 REST 资源动词（`POST /api/v1/session/{id}/run` 等）与 WS `ClientFrame` 在产品 TUI 路径上 MUST 停用。实现上可先删产品 REST，或让其 410；不得与 RPC 双语义并存。

`ip9`（Approve/Subscribe 留 WS 层）废止：审批与订阅都是 RPC 方法，进同一 dispatch。

## 3. 体验不变、模型改变

用户仍看到流式正文、Esc 中止、审批弹层。TUI 侧从 `driver.run() → EventStream` 改为：发 `prompt` request、收 notifications、另发 `abort` / `approve`。相关用 `id` 或 `session_id`，不绑死某一根 socket 任务。

## 4. 延后草案：方案 A（不实现）

目标体感：一条 `xylitol` 起 Host+TUI；TUI 仍是 **真** HTTP client，连本进程 loopback（不是 `InProcessDriver`）。

未决：

- 绑定已被占用 → attach 已有 Host，还是失败。
- TUI 先退、Host 仍听 → **孤儿 Host**。谁杀：显式 `server stop`、引用计数、还是退出时问。
- Host 先死 → TUI 重连还是退出。
- Ctrl+C 发给谁。
- default 仍含 server；子进程 Host vs 同进程两个任务。

这些不进本票 live specs。落地时新 change，depends_on 本票。

## 5. 与后续票

| 票 | 本票之后 |
|---|---|
| c2302 | 暂停。重写为：绑定占用、多 session 写者、框架（salvo 仍可）——**载本信封**，不是 WS Command/Event |
| c2303 | 不再「embed 默认」。最多 `serve` 改名、`--host/--port` 形状、A 的 CLI 糖 |
| c2304 / c2305 | 符合性 / ACP 仍后置；ACP 不共用本产品信封 |

## 6. 测试边界（seam）

复用现有 harness，不另开脱离 `.feature` 的边界：

- CLI 子进程：`tests/features/cli-entry.feature`（`xylitol tui` 未在听失败；Host 在听则进入 TUI）
- `llmanspec/specs/app-tui/app-tui.feature` `@req:tui2` 改为 attach 默认
- `server-core` 现有 `.feature`：产品路径改为 RPC+SSE，不再断言 REST run / WS Command
- print / `xylitol::embed` 现有 InProcess 测保留
- `cargo test --test bdd`；满闸 `just qa`
