# Design：四象限信封 + 独立 Host

## 1. 拓扑（本票交付：C）

```text
xylitol server run     →  Host 听 127.0.0.1:18790
xylitol tui            →  只当 client：POST unary + WS 下行
xylitol print …        →  仍 InProcess（一次性，无多 client）
库 xylitol::embed      →  仍 InProcess（给 crate 用户，不是产品 TUI）
```

未在听：TUI 非零退出，提示启动 Host。禁止静默落到 InProcess。

会话状态、写者位、待审批、journal、fastrace 在 Host。TUI 崩溃不杀 Host。

## 2. 信封（投递可换，词表不可双轨）

对齐 DSH 四象限，**不用** JSON-RPC 2.0（他们否决过复用自家 stdio JSON-RPC：数字码会塌成一个 fallback）。

| 象限 | 谁发起 | 本票物理（C） |
|---|---|---|
| ClientRequest | client | `POST /api/<method>`，body 含 `type=client-request`、`rpcId`、`method`、`payload` |
| ServerResponse | host 答 ① | 该 POST 的 HTTP 200 body：`type=server-response`、回显 `rpcId`、`result`（ok/error） |
| ServerRequest | host | **WebSocket 文本帧**（session 事件、审批/问卷 requested）。可应答帧用稳定 `rpcId`；纯推帧每次新铸 |
| ClientResponse | client 答 ③ | `POST /api/respond`，回填同一 `rpcId`；HTTP 体是载体回执，不是第二层 RpcMessage |

方法名与 payload 形状见 `research/method-table.md`（snake_case，对齐现行 Command serde tag）。线信封 MUST 是上述四象限。Command/Event 闭集（c2301）成为 **payload / 帧内容**，不再是「这条 WebSocket 上的 serde 枚举」。未知方法信封解析失败（保留缺口不进 v1 map）。

WS / UDS / UDP：只允许作为**同一信封**的另一种投递。本票不实现 UDS。禁止为本机再发明第二套词表。

现行 REST 资源动词（`POST /api/v1/session/{id}/run` 等）与 WS `ClientFrame` 在产品 TUI 路径上 MUST 停用。实现上可先删产品 REST，或让其 410。

`ip9`（Approve/Subscribe 留 WS 层）废止：审批与订阅都是信封方法/帧，进同一 dispatch。

### 通道纪律（对齐 DSH 2026-08-04）

- 网络：unary + respond **永远 HTTP POST**；下行 **永远 WebSocket**，该套接字 **不收** 业务上行。
- 进程内同构（测试 / 日后 print 若要走信封）：可用 in-process fetch + SSE 编解码证明信封与通道无关。产品 TUI **不**用 SSE。
- 网络 GET 下行路径只接受 Upgrade。MUST NOT 给浏览器留 SSE 兼容回退（会分叉）。

TUI 不吃浏览器六连接配额，但仍用 WS 下行，避免 Web 开闸时再换一次通道。

## 3. 体验不变、模型改变

用户仍看到流式正文、Esc 中止、审批弹层。TUI 从 `driver.run() → EventStream` 改为：发 `prompt` request、收 `ServerRequest` 帧、另发 `abort` / `respond`。相关用 `rpcId` 或 `session_id`，不绑死某一根 socket 任务。

## 4. 类型复用 + typed client + specta 闸

Eden 是 **同一 TypeScript 进程** 里用类型推断、零 codegen。Host 是 Rust、未来 Web 是 TS，做不到 Eden 那种零生成。等价物：**Rust 类型 SSOT + TUI 直接 `use` + specta 给 Web**。

### 4.1 一份方法表

`research/method-table.md` 是本票实现清单。unary payload = 现行 `Command` 变体字段（线层 `id` 收成信封 `rpcId`）。不要再抄 `PromptParams`。

### 4.2 一个 trait、两个 carrier

```text
HostClient
  unary(method, payload) → ServerResponse     # POST 或进程内
  respond(rpcId, payload) → 载体回执          # POST /api/respond
  mux() → Stream<ServerRequest>               # WS 或进程内通道
```

| 实现 | 谁用 |
|---|---|
| `InProcessClient` | 符合性 / 单测；与 print 现有 `InProcessDriver` 可并存（print 本票不必换信封） |
| `HttpWsClient` | **产品 TUI**。reqwest unary + tungstenite 下行。无监听器 → 失败，禁止改 InProcess |

`XyDriver` 收成 Host 内应用缝 / in-process 调度；跨进程不跳过信封直调 Driver。废止 `ip9`（Approve/Subscribe 留 WS 层）。

c2302 落地前：`HttpWsClient` 形状本票就交；连不上就按产品失败。不得用 InProcess 冒充产品路径。

### 4.3 specta（本票落地；无 Web UI）

- 四象限四成员、`RpcResult`/`RpcError`、方法表 payload/value、`Event`、下行帧 payload：`#[derive(specta::Type)]`。
- 导出检入 `clients/typescript/bindings.ts`（目录名可在落地时微调；MUST 检入、MUST 可 `just` 重生）。
- `scripts/check_*.py` 重生后 diff；经现有 `check-scripts` glob 进 `just qa`。漂移 → 红。
- **不是** Web SPA、不是手写 TS `AbstractApiClient`（后置草案）。本票只保证 Rust SSOT 与 TS 类型文件同步。

禁止：rspc；ts-rs 当方法表；OpenAPI/AsyncAPI/salvo oapi 当 SSOT。细则 `research/type-sharing.md`。

## 5. 观测

fastrace + `log` 仍只在 Host 内核（ReAct / 工具 / LLM）。HTTP 皮只记信封（method、rpcId、时长、HTTP status），**禁止**引入 salvo `Logger` 的 `tracing` 双栈。可留 envelope tap 座位（无订阅者零成本），给日后 Langfuse/诊断面板。

## 6. 延后（不进本票 live specs）

后置草案（避免只写在本节遗忘）：

| id | 记什么 |
|---|---|
| `c2315-add-loopback-host-tui` | 方案 A：一条命令 loopback，TUI 仍是真 HTTP+WS |
| `c2310-add-web-ts-client` | 薄 TS 客户端吃 `bindings.ts`；Vite SPA 另议 |
| `c2320-add-salvo-oapi-docs` | oapi 仅 unary 调试文档，非 SSOT |
| `c2325-add-cross-surface-actions` | 跨面公共动作 id（expandNearest 等） |

## 7. 与后续票

| 票 | 本票之后 |
|---|---|
| c2302 | salvo 载体：按方法表展开 POST `{method}` + WS 下行；绑定占用、多 session 写者 |
| c2303 | CLI `serve`/`--attach` 形状；握手 `host.describe` |
| c2304 | `InProcessClient` vs `HttpWsClient` **同方法表**双跑 |
| c2305 | ACP 外层译进方法表；不共用产品信封 |

## 8. 测试边界（seam）

复用现有 harness，不另开脱离 `.feature` 的边界：

- CLI 子进程：`tests/features/cli-entry.feature`（未在听失败；Host 在听则进入 TUI）
- `app-tui.feature` `@req:tui2` 改为 attach 默认
- `server-core` 现有 `.feature`：产品路径改为 POST + WS 下行，不再断言 REST run / 全双工 WS Command
- print / `xylitol::embed` 现有 InProcess 测保留；方法表 / specta 导出单测进 crate 测
- `scripts/check_protocol_ts_bindings.py`（名可微调）进 `just qa` 的 `check-scripts`
- `cargo test --test bdd`；满闸 `just qa`

## 9. 旧分支 landing 哪些可抄

`sdd/c2290-update-standalone-host` 的 **attach 默认 / 未在听失败 / 一写者挂 Host** 文本可在新 landing 时重写进 live specs。其中 JSON-RPC 2.0、SSE 为真源、`pa-rpc1` 焊 SSE 的句子 **全部丢弃**。
