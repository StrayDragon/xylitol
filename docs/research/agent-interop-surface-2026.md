# 多语言互操作面：独特点不在 codec，在「标准 Agent + 更富 Host」（2026-09）

> **性质**：跨 change 主题级耐久底稿。不是实现计划，不改 live specs。
> 前序：Fory 不适合产品编解码（`[apache-fory-fit-2026.md](./apache-fory-fit-2026.md)`）。本文回答「怎样让非 Rust 端不再为 Command/Event 付专有税，并形成相对其它 coding agent 的独特能力」。成本不作筛选条件。

## 决策（探索结论）

**2026-09-26 锁定**（JSON 真源 + 第一方面信封目标 JSON-RPC；**ACP 本轮不做**；Fory/gRPC 不当产品真源）：

1. **痛点诊断**：开发成本高的不是 JSON vs 二进制，而是 **专有词表 + 专有信封**。现行线协议已经是 JSON；缺的是业界会的那套方法名。
2. **第一方面**：走 **JSON 文本**；目标把四象限外层改成 JSON-RPC 2.0 形状，**通道仍拆开**（POST unary + WS 只下行）。词表仍是 Command/Event。
3. **不要** Fory / 自研 Fory-over-HTTP / Fory-gRPC 当产品编解码或独特点。可选程序 gRPC 若做，用标准 protobuf，且 **不**换 TUI。
4. **不要**把 MCP Server 当主互操作面。
5. **ACP 后置**：`c2305` 保持搁置，不纳入本轮 propose/apply。外人面互操作仍记在本文，但不开工。
6. **落地顺序**：抽出四象限 JSON 的单一 encode/decode（不改约）→ SDD 改 `protocol-app` 做 JSON-RPC 双读/翻客户端。ACP 不排进这条队列。



## 问题重述

用户要的独特能力 ≈「有一种约定好的结构，能和大多数语言交互」。现行 `protocol::Command` / `Event` + 四象限信封对 Rust 极顺，对 TS/Python/Go 等于再实现一座 Host。产品 TUI 路径合约禁止以 JSON-RPC 2.0 为产品协议（`protocol-app`）。

## 业界已经有的「约定结构」


| 面                    | 谁在用                                                                                                                                     | 形状                                                   | 覆盖                                                                                                                  |
| -------------------- | --------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| **ACP**              | Zed 客户端；Gemini CLI / Claude Code / Codex / OpenCode 等以 Agent 或 adapter 接入；官方 SDK：TS（npm 周下载约 7M）、Python、Rust、Kotlin、Java；JSON Schema 发布 | JSON-RPC 2.0，stdio 为主；v2 draft；HTTP/WS 有 RFD         | `initialize` / `session/{new,load,prompt,cancel}` / `session/update`；权限、FS、terminal、plan、slash、usage、compaction RFD |
| **Codex App Server** | OpenAI 自己的 VS Code / 多语言客户端                                                                                                             | 也是 JSON-RPC 2.0 JSONL；**专有**方法表（`thread/`* `turn/*`） | 比 MCP 富（diff、审批、fork）；仍是 Codex 方言                                                                                   |
| **MCP Server 模式**    | Claude Code / Codex 都提供                                                                                                                 | JSON-RPC 工具口                                         | 把 agent 当 callable tool；会话语义弱                                                                                       |
| **厂商 SDK**           | Cursor `@cursor/sdk`、Claude Agent SDK                                                                                                   | 语言绑定，不是跨编辑器标准                                        | 绑一家 runtime                                                                                                         |


编码 agent 互操作的「LSP 时刻」已经发生，标准名是 **ACP**，不是再发明 xylitol-RPC，也不是 Fory。

## xylitol 已有、却锁在专有信封里的资产

这些能力多数 ACP 正在标准化或已有 RFD，xylitol **已经打过一轮产品**：


| xylitol                                              | ACP 邻近                                                 |
| ---------------------------------------------------- | ------------------------------------------------------ |
| `ServerHello` + `host.describe` + `PROTOCOL_VERSION` | `initialize` 能力协商                                      |
| `Prompt` / `Abort` / `Subscribe` + `session/event`   | `session/prompt` + `session/update` + `session/cancel` |
| `approval/requested` / `question/requested` 反向通道     | `session/request_permission` / elicitation             |
| 写者租约、单写者、`resync_required`                           | 会话续上 / 缺口（ACP resume replay RFD）                       |
| compaction / usage / queue / todo                    | compaction RFD、session usage 已稳定、plan/todo             |
| `GetCommands` / 产品 slash                             | slash commands                                         |
| `Fork` / session tree                                | session fork RFD                                       |
| HTTP POST + WS 下行                                    | transports WG + HTTP/WS RFD                            |


**执行位点差**（发展点，不是障碍）：ACP 默认 Agent 是编辑器子进程，FS/terminal 是 **Client 方法**（编辑器拥盘）。xylitol 默认 Host 拥工具与工作区。可协商：attach Zed 时走 client FS；TUI/gpui 时走 Host 工具。这是 ACP `capabilities` 该表达的，不是换序列化能解决的。

## 三层公开面（建议的产品形状）

```text
第一方面（TUI / gpui / Print / embed）
    └── 四象限信封 + Command/Event     ← 丰富度真源，保持 Rust 闭集

第二方面（Zed / VS Code ACP / JetBrains / 任意语言脚本）
    └── ACP Agent 方言                 ← 用官方多语言 SDK；方法子集 + `_` 扩展

第三方面（自动化 / 其它 agent 当工具调 xylitol）
    └── 可选 MCP Server                ← 窄、无会话深度；零成本默认关
```

第一方面继续遵守「产品路径不是 JSON-RPC」。ACP 是 **附加方言**，类比 MCP 是附加工具方言，不是替换 TUI 真源。

### 方言里 xylitol 仍然独特的部分（用 ACP 扩展，而不是再发明信封）

ACP 基线没有、xylitol 有产品故事的：

- 轮内 **steer / follow-up / 队列条**
- **session tree travel**、entry label
- 冷恢复快照 vs journal 禁回放
- 写者租约
- `/reload`、loaded-resources、Trust 门禁呈现

这些走 ACP 的 `_xylitol/*` 自定义方法 + `initialize` 里广告 capabilities。第一方面继续用原生 Command。外部语言：**能跑 ACP 基线就能用**；要队列/树旅行再认 xylitol 扩展。

DeepSeek Harness 的反证：他们的 ACP 脸 **故意零私有方法**，独特点留在 Web/SDK。更稳的落地顺序是 **先标准 ACP v1 子集**（能接官方 SDK），扩展包第二阶段再广告；不要一上来把四象限全词表塞进 ACP，否则又变成 Codex 式方言。

## 为什么这比「生成自己的 TS SDK」更独特


| 策略                                    | 其它语言成本           | 独特点                                                                          |
| ------------------------------------- | ---------------- | ---------------------------------------------------------------------------- |
| 只生成 xylitol Command/Event 的 TS/Python | 降，但仍学 xylitol 方言 | 无。Codex App Server 已是这条                                                      |
| 只做 MCP Server                         | 极低               | 无。功能被削成 tool                                                                 |
| 换 Fory/Protobuf                       | 几乎不降（仍要懂词表）      | 无                                                                            |
| **做 ACP Agent + 把富语义回馈标准（RFD）**       | 基线 ≈ 0（官方 SDK）   | **一份 Rust Host：TUI/gpui 原生丰富 + 任意 ACP 编辑器可接**；有机会把 compaction/队列写进标准，而不是永远私有 |


忽略成本时，还应并行：从 `protocol::wire` **投影** JSON Schema / 官方 SDK（specta 一类），给「就要 xylitol 全词表」的嵌入方。这是卫生，不是独特点。

WIT / wasm 组件仍是 **工具/插件沙箱**（已写在 `ui-runtime-tradeoffs-2026.md`），不是 UI 客户端 ABI。

## 发展点（忽略成本的路线，非进度板）

1. **ACP Agent 适配器**：Host 侧把 `XyDriver` 投影到 ACP 方法；stdio 先（编辑器拉起进程），再跟 transports RFD 对齐 HTTP/WS（可与现行 mux 并列，不替换）。
2. **执行位点协商**：`fs`/`terminal` 由 client 提供 vs Host 工具拥盘；TUI 路径不变。
3. **富语义扩展包**：steer/queue/tree/lease 用 `_xylitol/`*；稳定后考虑 ACP RFD（compaction 已有社区 RFD，可对齐而不是分叉）。
4. **注册表**：ACP Registry 上架，让 Zed/VS Code 扩展「加 xylitol」零协议工作。
5. **原生词表投影**：Command/Event → JSON Schema，CI 对拍；OpenAPI 仍可保持信封级调试文档，避免第二词表。
6. **可选 MCP Server**：给「只想调一轮」的编排器；不承诺 diff/queue parity。



## 合约含义（若立项）

- `protocol-app`「产品 TUI MUST NOT 以 JSON-RPC 2.0 为产品协议」**仍然成立**。
- 新 capability 描述 **ACP 方言** 为后置/可选 attach，不得让产品 TUI 改说 JSON-RPC。
- 未知 ACP 方法/通知按 ACP 扩展规则忽略；xylitol 未知事件降级纪律平行适用。



## TUI ↔ Host 现在是不是 JSON-RPC？

**不是 JSON-RPC 2.0。** 判断「主流是 JSON-RPC」对 **ACP / MCP / Codex App Server / DeepSeek SDK** 成立，对 **产品 TUI attach** 不成立。

| | JSON-RPC 2.0（ACP/MCP/Codex SDK） | xylitol 产品 TUI ↔ Host |
|---|---|---|
| 外层字段 | 必有 `"jsonrpc":"2.0"`，`id` / `method` / `params` | `"type": "client-request"` 等四象限 tag；`rpcId`；**无** `jsonrpc` |
| 错误 | 数字码（-32700…）；`error` 与 `result` 互斥 | `RpcResult.ok` + 字符串 `code`/`details`；HTTP 200 表示信封解析成功 |
| 传输 | 通常 **一条** stdio/JSONL 双向流 | **拆开**：unary = HTTP POST `/api/{method}`；下行 = WS **只收 ServerRequest**；反向应答 = POST `/api/respond` |
| 握手 | `initialize` 方法 | WS 首帧 `ServerHello { protocol }`，不匹配即断 |
| 合约 | 业界默认 | `protocol-app` **MUST NOT** 以 JSON-RPC 2.0 为产品协议；BDD 断言 body 不含 `"jsonrpc"` |

它看起来「也是 RPC」（有 method、有相关 id），但信封、错误模型和通道切分都是自研四象限。TUI 走的是 `src/protocol/wire/envelope.rs` + `src/app/server/http.rs`（`Message::text`），不是 JSON-RPC 客户端。

因此：要接「主流」，是 **加一条 JSON-RPC 方言**（优先 ACP），不是把 TUI 改成 JSON-RPC。仓库里已有搁置提案 [`llmanspec/delayed-changes/c2305-update-acp-provider-adapter/proposal.md`](../llmanspec/delayed-changes/c2305-update-acp-provider-adapter/proposal.md)：适配器译成同一 dispatch，**不共用**产品信封。

## DeepSeek Harness 可借鉴处

对照仓：`../deepseek-harness`（2026 开发者预览）。Cordis「一切皆插件」与 xylitol「非插件市场」**产品定调相反**，不搬内核。值得搬的是 **同一套 agent 核、多张公开脸**：

| dsh 脸 | 协议 | 给谁 |
|---|---|---|
| `web` | 浏览器应用（内部 RPC，不是给外人的标准） | 人 |
| `--profile sdk` | **自有** JSON-RPC stdio（`dsh-sdk-jsonrpc-server`）；Python SDK **拉起**同版本 `dsh --profile sdk` | 程序 |
| `--profile acp` | **标准 ACP v1** JSON-RPC stdio；**不加**私有 method / `_meta` | Zed/脚本/官方 ACP SDK |
| `headless` | 无服务一次性跑 | CI |

他们把 ACP 写成 **automation-only**：只暴露标准语义更新（消息、thought、通用 tool 生命周期、配置、usage）；DSH 自己的卡片/plan/title/todo/terminal/elicitation **故意不下 ACP 线**。`session/resume` 恢复日志但 **不回放**旧 update。stdout **只**准协议帧。

对 xylitol 的启示（忽略成本也适用）：

1. **脸分开，核共用** — 与 c2305 一致：TUI 四象限、ACP 另一进程/另一入口、dispatch 同一。
2. **对外 ACP 先做「标准子集」** — DeepSeek 用「零私有方法」换官方 SDK 零摩擦；独特点放在第一方面（TUI/gpui），不要一上来就把 steer/租约塞进 ACP 搞成又一个方言。扩展可以第二阶段。
3. **程序 SDK ≠ 标准 ACP** — 他们同时养自有 JSON-RPC SDK 和 ACP。xylitol 若只要一条外人脸，优先 ACP（标准）；自有 SDK 投影是加分项。
4. **不要学** everything-is-plugin、默认 Web、Electron 再包一层。

## 若把产品面也改成 JSON-RPC（重构假说）

「Server 说 JSON-RPC，大家都能连」把三件不同的事叠在一起。忽略成本时也应拆开选，不能只换信封。

| 层 | 现在 | 改成 JSON-RPC 会怎样 |
|---|---|---|
| **信封** | 四象限 tag + `rpcId` + `RpcResult.ok` | 通用库能 parse；**方法名仍是 xylitol** |
| **词表** | `Command`/`Event` 闭集 | 不改则仍是 Codex 式方言；改成 ACP 才接官方 SDK |
| **通道** | POST unary + **WS 不收业务上行** | JSON-RPC 默认一条全双工流；这正是 `protocol-app` 目的行禁止的「全双工 WS 外层」 |

四象限不是「还没来得及做成 JSON-RPC」。合约把反向 RPC（审批/问卷）钉在 `client-response` + POST `/api/respond`，禁止把业务上行混进 WS（`r1700` / `r1709`）。JSON-RPC 2.0 的自然形状是单管道 peer：请求、应答、通知、反向请求全在一条 JSONL/WS 上。要产品真源改口，等于**撤回这条通道不变量**，不只是字段改名。

三条互斥路径（忽略成本）：

1. **只换信封、词表不动** — TUI/gpui 改说 `{"jsonrpc":"2.0",...}`，方法仍是 `Prompt`/`Steer`/写者租约。其它语言省掉自研信封，仍要学 xylitol 方法表。这是 Codex App Server。独特点：无。
2. **产品真源改成 ACP 词表** — TUI 变成 ACP client。外人零摩擦；steer / 写者租约 / 会话树要么进 `_xylitol/*`，要么砍掉。执行位点还要反过来（ACP 默认 Client 拥盘，TUI 默认 Host 拥工具）。DeepSeek **没有**让 Web UI 走 ACP。
3. **Server 另开 JSON-RPC 脸，TUI 不动** — 与 `c2305`、DeepSeek profiles 同构。产品路径继续四象限；外人连 `--profile acp`（标准子集）或可选自有 SDK JSON-RPC。这才是「这样 server 能被其它语言连」且不拆 TUI 通道纪律。

结论：**「重构到 JSON-RPC」若目标是跨语言，必要的是词表面（ACP），不是把 TUI 改口。** 若目标是第一方面自己也用业界信封，那是撤回 `protocol-app` 的独立决策，跨语言收益很小。

## 路径 B（只换信封）渐进切法与门禁是否够

前提：词表仍是 Command/Event；**不**放开全双工 WS（审批仍 POST `/api/respond`）。这是「JSON-RPC 形状的字节 + 仍拆开的通道」，不是一本正经的单流 JSON-RPC peer。通用 jsonrpc 库能 parse unary；反向 RPC 仍要手写「应答走另一条 HTTP」。

映射要点（会撞现有合约）：

| 现四象限 | JSON-RPC 2.0 近似 | 摩擦 |
|---|---|---|
| `rpcId` | `id` | 可机械映射 |
| `method` + `payload` | `method` + `params` | `writerToken` 现是信封字段，要进 `params`/`_meta`/header |
| `RpcResult { ok, value, error:{code:string} }` | `result` XOR `error:{code:number}` | **r1696 禁止 JSON-RPC 数字码**；须改约或把 xylitol 字符串码塞进 `error.data` |
| `ServerHello { protocol }` | 无 id 的 notification | `PROTOCOL_VERSION` 现为 1，改信封应 bump，旧 TUI 靠 mismatch 死掉而不是混跑 |
| WS 只下行 `ServerRequest` | 同 socket 上的 jsonrpc request | 应答仍不在该 socket 上回，**非标准** |

### 渐进（双读单写 → 翻客户端 → 再删旧解析）

1. **先改合约方向**（SDD）：`protocol-app` / `server-core` / `layer-architecture` 把「禁 JSON-RPC 2.0」改成「产品信封为 JSON-RPC 2.0 形状；通道纪律不变（禁全双工 WS 外层）」。不改这条就动手，现有 BDD 会把重构当回归打掉。
2. **Host 双读**：HTTP/WS 同时接受 `type:client-request` 与 `jsonrpc:2.0`；应答跟请求方言。`PROTOCOL_VERSION` 暂不 bump。旧 TUI 继续绿。
3. **编解码收到 `HostClient` 实现里**：trait（`unary` / `respond` / `mux`）保持内部 `RpcResult`，只让 `HttpWsClient` + `http.rs` 换 codec。`InProcessClient` 不动——print/嵌入本就不走字节信封。
4. **对拍闸**：同一批 HostClient 调用，四象限字节 vs JSON-RPC 字节必须得到同一 `RpcResult` + 同一下行 method/payload（不含外层字段名）。没有这张表就不要翻 TUI。
5. **翻 `HttpWsClient` 只发 JSON-RPC**，bump `PROTOCOL_VERSION`。旧客户端死于 hello mismatch（已有 ath44）。
6. **删旧解析**。双读窗口结束。

不要在同一步里改错误模型、放开 WS 上行、或改 Command 名。那三件事各自是独立回归源。

### 现有门禁够不够锁行为？

**不够。** 现在的验收测是按「禁止 JSON-RPC」写的，足以挡住误改，**不足以**证明「换信封后语义不变」。

会直接红、且本来就该先改约的（钉死旧字节）：

- `protocol-app`：`four-quadrant-envelope-shape`（字面 `{"type":"client-request",...}`）；`unary-stable-error-envelope`（`!body.contains("jsonrpc")` + `ok` 字段）
- `tests/bdd/steps_server.rs` 里手拼 `"type":"client-request"` 的幂等/unary 夹具
- `server-core`「下行信封 MUST 为四象限 ServerRequest（type、rpcId、method、payload）」
- `oapi.rs` 五件套 schema（`type` const）
- 各 capability purpose 行与 `r1701`/`r1709`/`r1778` 的「MUST NOT JSON-RPC」

能保住 **词表/通道**（换信封后应继续绿、不要改断言含义）的：

- Command/Event serde 往返、方法表登记、dispatch 进 Driver
- 写者租约、反向 RPC 第一应答、last_seq / `resync_required`、queue_stats 不占写者
- **WS 不收业务上行**（这条是通道不变量，路径 B 必须留下）
- ath44 hello mismatch 致命、不重试
- 大量经 `HttpWsClient` 的 server-core 真线场景（写者、审批、资源下行）——它们走 trait，不手拼 JSON；翻 codec 后它们是真回归网
- TUI headless / `just test-tui`：多数不打 HTTP 信封，**挡不住** `HttpWsClient` 编错

缺口（现在没有、翻客户端前必须补）：

1. **方言对拍矩阵**（上节第 4 步）——最大洞
2. 把 `steps_server` 手拼 JSON 收进「按方言编码」助手，避免夹具与产品 codec 各写一份
3. 错误模型若坚持 JSON-RPC 数字码：现网只断言「无 jsonrpc、code 是字符串」，没有「业务码稳定、与 dispatch 同源」的独立闸
4. `just qa` 不含 `qa-e2e`；PTY/tmux 不替代信封对拍

结论：路径 B **可以**渐进，缝在 `HostClient`；但要先改约、加双读对拍，再翻 TUI。现有 `just qa` **不能**原样当「重构前后行为锁」。把钉信封的场景改写成钉 **通道 + 方法表 + 方言对拍** 之后才够。

## 重构前可提前动的代码（不换协议）

这些不改 `protocol-app` MUST，适合当「双读切片」的前置卫生，避免到时候夹具和产品 codec 各写一份：

1. **抽出 `WireCodec`** — 已落到 `src/protocol/wire/codec.rs`；`HttpWsClient`、mux hello/下行、unary/respond 入站、server BDD 夹具走同一 encode/decode。字节仍是四象限 JSON。**尚未**走 codec 的：unary **出站**仍是 Salvo `Json(RpcMessage)`（与 `encode` 同一 serde，双写时再收口）。
2. **夹具走同一编码器** — `tests/bdd/steps_server.rs` 的 `post_unary_rpc_id` 手拼 `"type":"client-request"`。改成调用产品 codec 后，双读/翻方言不会出现「测试说四象限、产品说 jsonrpc」的假绿。
3. **HostClient 级金丝雀** — 断言 `unary`/`respond`/`mux` 的 `RpcResult` 与下行 method/payload，**不要**断言外层字段名。今日真线场景已经走 `HttpWsClient`，差的是「同一调用、两种字节」矩阵（仍属双读切片，要改约才能在生产双读）。
4. **映射表写下来，先不写代码** — `rpcId`↔`id`，`payload`↔`params`，`writerToken` 放哪，`ServerHello` 怎么变成 notification。错误模型（字符串码 vs JSON-RPC 数字码）单独决策，不要捆进 codec 抽取。

不要提前：`PROTOCOL_VERSION` bump、WS 改 binary、给 `Event` derive Fory。那是换通道或换词表，不是信封卫生。gRPC 若做，是**另一张脸**（见下），不要绑进 codec 抽取。

## 「Fory RPC」不能替代这条 JSON-RPC 假说

Apache Fory 的 RPC 是 **gRPC + Fory 载荷**（`foryc --grpc`），不是 JSON-RPC 的二进制版。TUI 若「直接反序列化 Fory 二进制」，最多是现行 WS **改 binary 帧 + native ForyUnion**（仅 Rust↔Rust）。跨语言 xlang 仍拒绝多字段 Event 变体。流式热路径实测见 [`apache-fory-fit-2026.md`](./apache-fory-fit-2026.md)：JSON-RPC 解码在 80 tok/s 下约占一核 0.003%，不构成换 codec 的证据。

忽略成本时 **可以**再开一张 gRPC 脸，但：

- 编辑器/官方 SDK 说的是 ACP JSON-RPC，不是 gRPC；gRPC **接不上** Zed。
- 若开 gRPC：用 **标准 protobuf + tonic**（`grpcurl`），不要 Fory-gRPC（与 protobuf 客户端不互通，调试更差）。
- TUI 仍走 JSON；gRPC 只给「服务调 Host」这类程序方，且仍要自建方法表，除非方法名就是 ACP。

## 相关落点

- 线协议：`src/protocol/wire/`；信封禁 JSON-RPC：`llmanspec/specs/protocol-app`
- 多 client：`docs/architecture/库与多客户端.md`
- 开闭：`docs/architecture/扩展与开闭.md`（挂既有轴，不新开插件平台）
- ACP：[https://agentclientprotocol.com/](https://agentclientprotocol.com/) ；schema/SDK：[https://github.com/agentclientprotocol/agent-client-protocol](https://github.com/agentclientprotocol/agent-client-protocol)
- Codex 为何不用 MCP 当 IDE 主面：[Unlocking the Codex harness](https://openai.com/index/unlocking-the-codex-harness/)
