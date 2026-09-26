# Apache Fory 对 xylitol 的适配性（2026-09）

> **性质**：跨 change 主题级耐久底稿。不是实现计划，不改 live specs。
> 产品方向正文：[`../architecture/库与多客户端.md`](../architecture/库与多客户端.md)、[`../architecture/远程体验与线协议.md`](../architecture/远程体验与线协议.md)、[`../roadmaps/Gpui桌面客户端.md`](../roadmaps/Gpui桌面客户端.md)、[`ui-runtime-tradeoffs-2026.md`](./ui-runtime-tradeoffs-2026.md)。

## 决策

**不引入 Apache Fory 作为产品编解码。** 不替换四象限信封（envelope）的 JSON、不替换会话 JSONL、不替换 MCP / hook 的 JSON Value 口。Web / TS 管控台若复活，仍先打现行 JSON 信封；类型层从 Rust protocol 投影（手写或 specta/ts-rs），**不要**把调试 OpenAPI 升格成 SDK 真源。

## 问题

xylitol 主业务在 Rust，后续可能有多种 client（下一产品端是 gpui；Web/TS 管控台搁置）。Fory 宣传跨语言对象图序列化（Rust + JS/TS）。它是否该成为 Host↔client 的互操作层？

## xylitol 侧事实

| 接缝 | 现行形状 | 约束 |
|---|---|---|
| 线协议 | 四象限信封；Command / Event 为载荷；`serde` JSON；WS **文本帧** | `src/protocol/wire/envelope.rs`；`src/app/server/http.rs` `Message::text` |
| 应用协议 | `XyDriver` + `XyEvent`；跨进程不跳过信封 | `src/AGENTS.md` 三层契约 |
| 会话持久化 | JSONL；`ExportJsonl` / `ImportJsonl` 是产品命令 | `protocol-app` r1693 / r1699；AgentPart 自描述 JSON（r1711） |
| MCP / hooks | **必须** JSON Value | `src/AGENTS.md`「工具与钩子」 |
| 下一端 | gpui，与 TUI 同为 Rust，直接 `use` 协议闭集 | `docs/research/ui-runtime-tradeoffs-2026.md` |
| Web | 搁置；c2310 TS 客户端与 specta 导出已删 | 同上；`docs/roadmaps/Cloud-Agent与Web控制台.md` |
| 调试 | `GET /openapi.json` + `GET /docs` Scalar；**不是**客户端生成真源 | `docs/architecture/远程体验与线协议.md` |
| 未知事件 | 可降级忽略，禁止 panic | protocol-app r1719 |

热路径是模型 / 工具 / TTY，不是信封 JSON 编码。

## 流式热路径实测（2026-09-26，本机临时 crate）

探针：`/tmp/xylitol-codec-bench`（**不进仓**；`fory` 1.7.5 native+xlang flatten vs `serde_json` vs `rmp-serde`）。N=50_000 帧，release，模拟 `session/event` + `text_delta`。JSON 路径保留 tagged Event；Fory 路径是**压扁 struct**（无枚举、无 `Value`）——这是「TUI 直接反序列化」的乐观形状。

「Fory RPC」官方形状是 **gRPC 传输 + Fory marshaller**（`foryc --grpc` / tonic），不是 JSON-RPC 的二进制孪生，也不能套在现行 POST+WS 四象限上当 drop-in。下面比的是 **TUI 解析一帧的 codec 成本**，不是 HTTP vs gRPC。

典型 token（8 字节文本）每帧：

| codec | 字节 | 反序列化 ns/帧 |
|---|---:|---:|
| 四象限 JSON | 160 | ~356 |
| JSON-RPC 2.0 JSON | 148 | ~352 |
| MessagePack（同一 serde Event） | 70 | ~179 |
| Fory native flatten | 85 | ~93 |
| Fory xlang flatten | 85 | ~93 |

80 token/s 时 JSON-RPC 解码约占 **一核的 0.003%**。Fory 大约快 4×、帧大约小一半，但省下的是已经看不见的预算。MessagePack 在**不改 Event 形状**的前提下拿走大半二进制收益。

xlang 对「现有 Event 多字段变体」：`register` 报 `NotAllowed("multi-field tuple and named enum variants are not representable in xlang mode")`。native `ForyUnion` 可以（Rust↔Rust）。因此「TUI 直接吃 Fory 二进制」只对 **同语言 native** 成立——而 TUI/gpui 已经是同语言 serde。要给 TS/Python 用 xlang，必须先把每个变体拆 message，不是换 WS 帧类型。

RSS 增量在同进程连续 case 间被分配器复用污染，**不要**拿表里的 RSS Δ 当产品证据。保留 5 万个已解码 JSON 对象可把进程堆到 ~50MB；真 TUI 是把 delta **折进**一段 String，不会堆 5 万个信封。

一轮 mock（hello + subscribed + 40× text_delta + tool 流 + approval + agent_end，共 53 帧）：JSON-RPC 合计 **7470 B**，可直接 `grep type`；Fory flatten 合计 **4457 B** 但 `args` JSON 被丢掉，样例是 `00ff1c00…` 十六进制，curl/Scalar/会话 JSONL 对不上。

## 建议（JSON-RPC vs Fory-over-HTTP vs gRPC）

Fory 自己的分层（不是我们发明的）：

| 他们叫什么 | 官方用途 | 调试 |
|---|---|---|
| **Fory JSON** | HTTP API、浏览器、日志、配置 | 就是文本 JSON |
| **Fory 二进制** | 对象图、引用身份、跨语言 schema 元数据 | 需 `ENABLE_FORY_DEBUG_OUTPUT` / 对拍测试，不是 Wireshark 可读 |
| **Fory gRPC** | `foryc --grpc`：传输是 gRPC，载荷是 Fory | **不是** protobuf 字节；通用 `grpcurl`/reflection **解不开** |

因此：

1. **不要自研「Fory RPC over HTTP」**。这既不是 JSON-RPC 标准，也不是 Fory 官方 RPC。调试比四象限更差（二进制 + 自研信封），跨语言还要撞 xlang union。Go 文档里的 HTTP `octet-stream` 示例是「把 Fory 当 body」，不是一套 RPC。
2. **第一方面继续 JSON。** 产品已经有 Scalar、JSONL 导出、未知事件按 `type` 丢弃。JSON-RPC 2.0 只换外层字段，**debug 手感几乎不变**（仍是文本、仍能 curl）；相对四象限的收益是「通用库能 parse 信封」，不是更快。流式 80 tok/s 解码约一核 0.003%，不构成换二进制的理由。
3. **外人面仍是 ACP（JSON-RPC stdio），不是 gRPC。** Zed/官方 SDK 说 ACP。开 gRPC 脸忽略成本也可以，但接不上编辑器生态。若真开 gRPC：用 **标准 protobuf + tonic**（`grpcurl`、reflection），不要 Fory-gRPC——后者故意与 protobuf 客户端不互通。
4. **TUI 不要改口 gRPC。** HTTP/2 + 证书/deadline 对 loopback 个人 harness 是加重量；反向审批已经是四象限 POST `/api/respond`。gRPC bidi 反而撤回「WS 不收业务上行」。
5. **以后若 JSON 真成热点**：同一 serde 词表上的 MessagePack，比 Fory 更接近「还能写调试双路」。

一句话：**标准用在脸上（ACP / 可选 JSON-RPC 信封），二进制只在测出热点之后当 codec 插件；不要用 Fory 同时当信封、当 RPC、当独特点。**

## Fory 侧事实（一手）

| 项 | 事实 | 来源 |
|---|---|---|
| 定位 | 多语言二进制对象图 + row format + 可选 JSON + IDL/gRPC | [docs](https://fory.apache.org/docs/) |
| 治理 | 孵化器毕业 TLP：2025-07-17（曾用名 Fury） | [毕业博文](https://github.com/apache/fory-site/blob/main/blog/2025-07-17-apache_fory_graduated.md) |
| Rust | `fory` 1.7.4 稳定 / 1.7.5-rc；默认 xlang；native 用 `.xlang(false)` | [Rust 指南](https://fory.apache.org/docs/guide/rust/)；[crates.io/fory](https://crates.io/crates/fory) |
| JS/TS | `@apache-fory/core` 1.7.5；**仅 xlang**；hps 为 Node 20+ 可选加速 | [JS 指南](https://fory.apache.org/docs/guide/javascript/) |
| JS 采用 | npm 周下载约 262（2026-09-26 快照）；依赖含 `node-gyp` | [npm](https://www.npmjs.com/package/@apache-fory/core) |
| 浏览器 | 文档主路径是 Node 示例 + gRPC-Web 生成客户端，不是「浏览器里直接当信封 codec」 | [gRPC overview](https://fory.apache.org/docs/next/grpc/) |
| **xlang union** | **一案最多一个值**；Rust 多字段 tuple/named 变体是 **native-only**；`xlang=true` 注册报错；不偷偷拆 struct | [external-types.md](https://github.com/apache/fory/blob/main/docs/object-serialization/rust/external-types.md)（Context7 摘录同源） |
| **IDL `any`** | 只允许 `bool` / `string` / `enum` / `message` / `union`；**不含** list/map 与其它原语 | [schema-idl.md](https://github.com/apache/fory/blob/main/docs/compiler/schema-idl.md) |
| IDL union | 可表达「每个 case 一个 message」的 sum type，生成 `enum Animal { Dog(Dog), … }` | [2026-03-09 IDL 博文](https://github.com/apache/fory-site/blob/main/blog/2026-03-09-fory_schema_idl_for_object_graph.md) |
| 基准 | 官方 Rust xlang 对象序列化常快于 protobuf / msgpack | [Rust xlang bench](https://fory.apache.org/docs/benchmarks/object-serialization/xlang/rust/) |

Fory 真正强的地方：共享引用 / 环 / 多态对象图、分析型 row、JVM 互通。xylitol 的 Command / Event 是**事件树**，不是对象图。

## 错配

1. **词表形状**。`Event::ToolStart { id, name, args }` 这类多字段标签枚举不能进 xlang。要采用 Fory，必须先把每个变体拆成独立 message 再放进 IDL `union`——那是重写协议，不是换 codec。
2. **开放 JSON**。信封 `payload`、`ToolStart.args`、MCP、hooks 需要任意 JSON。Fory `any` 不是 `serde_json::Value`。
3. **产品可读性**。JSONL 导出、Scalar 调试、未知事件按 `type` 丢弃，都依赖文本 JSON。二进制默认与这条产品线冲突。
4. **语言边界不存在**。gpui 与 TUI 同语言；为搁置的 TS 管控台预支编解码，违反「不为未交付能力堆抽象」。
5. **JS 运行时不匹配管控台**。周下载极低、`node-gyp`、hps 绑 Node。浏览器页打 HTTP/WS JSON 更便宜。

## 若 Web 复活：顺序

1. **保持 JSON 信封**。TS 类型先手写，镜像 `#[serde(tag = "type")]`；未知 `type` 丢掉；开放 JSON 字段用 `unknown`。
2. 类型漂移难忍 → 从 Rust `protocol::wire` 导出（specta / ts-rs；2026-08 已删的 c2310 可复刻）。生成物是投影，Rust 仍是真源。
3. **不要**把 `GET /openapi.json` 升格成 SDK：它只有信封五件套，payload 无 schema；单测禁止 per-method schema（第二词表）；WS 下行不是 OpenAPI path。合约：`server-core`「MUST NOT 作为客户端生成真源」。
4. 测出 JSON CPU / 体积热点 → **MessagePack / CBOR**（仍走 serde，枚举与 Value 可保留）。本机 TUI/gpui loopback 几乎碰不到这门槛。
5. **Protobuf + protojson** 仅在决定「IDL 为协议真源」、且 ≥2 个非 Rust 产品端成为日常时再比。这是换真源，不是换 codec。
6. Fory 排在 Protobuf 之后：union / any 更不合现有词表，JS 生态也更弱。

## 调试 OpenAPI 为什么升不成 TS SDK

`src/app/server/oapi.rs` 从方法表生成 unary path，schema **故意**停在 `ClientRequest` / `ClientResponse` / `ServerResponse` / `RpcResult` / `RpcError`。`payload` / `value` 的文档句是「shape per Rust protocol wire types」。`components_are_envelope_level` 断言不得加 per-method schema。

因此 `openapi-typescript` 生成的客户端对业务仍是 `unknown`。要让 OpenAPI 变得「可生成」，先得给 Command / Event derive `JsonSchema` 并写进 components——与 `src/AGENTS.md`「protocol 根类型默认只 serde；schemars 放 infra/config」相反，且制造第二词表。Web 要类型时走「从 Rust 导出」，不要改调试文档的职责。

## MessagePack / Protobuf 翻案门槛

| 门槛 | MessagePack / CBOR | Protobuf |
|---|---|---|
| 要解决的问题 | 同样 serde 模型，更密的字节 | 多非 Rust 端不漂移；字段号演进 |
| 类型模型 | 几乎不动 | 每个变体拆 message + oneof；SSOT 改嫁 `.proto` |
| 调试 / JSONL / Scalar | 双路或放弃文本 mux | protojson 可留文本；会话 JSONL 仍另套 |
| MCP / hooks | 仍是 JSON | 仍是 JSON；IDL 与 hook 永久双词表 |
| 触发 | profile 显示信封 serde 进热路径，或远程带宽成产品问题 | ≥2 个非 Rust 产品端成为日常，且手写/导出扛不住漂移 |

假问题不要用它们解：TS 缺类型 → 从 Rust 投影；gpui/TUI 要更快 → 瓶颈不在 JSON；schema 演进 → 现行是 `PROTOCOL_VERSION` 硬握手 + 未知事件降级，换字段号是另一套产品语义。

## 翻案条件

同时满足再打开，不跳步：

- Cloud-Agent / Web 管控台显式重立项，且需要**产品级** TS SDK（不是人手调试）；
- 官方 xlang 支持多字段 union **或** 团队接受把 Command / Event 全量拆 message；
- 有可观测证据表明 JSON 编码是瓶颈（当前没有）；
- 或出现真实的对象图 / 跨 JVM 负载（当前产品没有）。

## Fory 真正适合什么（对照 pickle / Thrift）

直觉「多语言 pickle」**对了一半**；「Thrift 替代」**只对 codec 那一层，不对 RPC**。

| | pickle / Kryo / Java 序列化 | Thrift / Protobuf | Fory |
|---|---|---|---|
| 单位 | 语言里的**对象图**（指针、共享、环、子类） | **值树 DTO**（无共享身份） | 两者都做：native ≈ pickle；xlang+IDL ≈ 带图语义的 Thrift |
| Schema | 无（类定义即契约） | IDL 先行 | 可无 IDL（对等类注册）或 `.fdl` |
| 跨语言 | 基本不能 | 一等公民 | xlang 一等公民；union 有「一案一值」限制 |
| RPC | 不管 | Thrift 自带；Protobuf 配 gRPC | 本身是 codec；gRPC companion 仍在做 |
| 安全 | pickle 不信任数据即 RCE | 字段白名单 | 要注册类型才反序列化（比 pickle 收敛，仍须受信边界） |

**适合 Fory 的具体场景**（xylitol 都没有）：

1. **JVM 领域对象出站，别的语言要原样图。** 例如 Java 订单服务里 20 条明细指向同一个 `Customer` 实例；Python 风控要这份图，不要变成「customer_id + 再查一次」或 JSON 里复制 20 份。这是官方相对 Protobuf 的主主张（共享引用 / 环是 schema 一等公民）。
2. **替换 Kryo / Hessian / Java 序列化，随后出现第二语言消费者。** 先 native 吃对象图吞吐，再 xlang。毕业声明里的「Pyfory 做 pickle 替代」也属这条。
3. **分析型部分反序列化。** Spark/Flink/Doris 类：一百万行只读其中几个字段，row format 不把整对象 inflate。xylitol 的事件流是全量投影给 UI，用不上。
4. **必须整图落盘的运行时状态。** 场景图、带 parent 指针的 AST、工作流 DAG 塞进 Redis 一个 blob，读回来指针身份还在。xylitol 会话是 JSONL 事件日志 + 快照，不是一张图。
5. **多态插件对象在受信任集群内搬运。** 基类/trait 上挂多种子类型，对端按注册表还原。这和 xylitol「未知事件降级 + 开放 JSON hook」相反：一边要闭集注册，一边要开放忽略。

**不适合（包括 xylitol）的具体场景：**

- 人要 curl / Scalar / grep JSONL 的 API（调试与导出是产品功能）。
- 闭集标签 DTO（`type` + 若干字段的事件/命令）——JSON 或 Protobuf 更省事。
- 浏览器薄客户端、开放插件 JSON、MCP。
- 「只是想给 TS 类型」——类型靠从 Rust 投影，不靠换二进制。

一句话：Fory 吃的是「**这张对象图就是领域模型，且要跨进程/跨语言仍是图**」。xylitol 吃的是「**这帧是给 UI 的事件，人要读，未知要丢，插件是 JSON**」。宽表编排、IR 快照、跨语言表数据不在本仓线协议范围内。

## 相关落点

- 线协议代码：`src/protocol/wire/`
- OpenAPI 调试文档：`src/app/server/oapi.rs`；合约 `llmanspec/specs/server-core`
- 架构 SSOT：`src/AGENTS.md`
- UI 运行时选型：`docs/research/ui-runtime-tradeoffs-2026.md`
