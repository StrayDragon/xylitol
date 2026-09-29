# 02 · xylitol 线协议现状盘点

> 基线:`main@f4b8d68e`。本篇回答两个问题:现有协议的强/动态分界线在哪(它决定 fdl 的管辖边界),以及引入二进制 schema 化序列化的接缝与被动面(改哪些、被动哪些)。行号为该基线快照。

## 1. 现有形态(自研 JSON-RPC 2.0)

| 组件 | 事实 |
|---|---|
| 信封 | `src/protocol/wire/codec.rs` 手写 JSON-RPC 2.0 构造/解析(纯 serde_json);`envelope.rs` 四象限 `RpcMessage`(ClientRequest/ServerResponse/ServerRequest/ServerHello),但其 serde type-tag **不是线方言**(codec.rs:1-6 注释明示,有测试锁死) |
| 分发 | jsonrpsee 0.26(仅 server-core + types):`src/app/server/rpc_module.rs:40-58` 把 registry 方法表注册进 `RpcModule`,unary 走 `raw_json_request`(4MB 上限,rpc_module.rs:73) |
| 传输 | salvo 0.95,`POST /rpc`(unary)+ `GET /rpc`(WS 下行 notification 流),默认 `127.0.0.1:18790`;Origin 检查仅放行 loopback(http.rs:336-349) |
| 客户端 | 自研:`src/app/core/host_client/http_ws.rs` tokio-tungstenite 手搓 WS + JSON-RPC,ping 20s / idle 40s,上行超时 30s |
| 嵌入 | `HostClient` trait 双载体:`HttpWsClient` 与 `InProcessClient`(broadcast 总线传内存对象,**零序列化**)——这是传输接缝的正解 |

**产品规则要点**(`docs/architecture/远程体验与线协议.md` + BDD `llmanspec/specs/server-core/server-core.feature`):事件闭集映射、未知下行可降级不崩、冷恢复走消息快照一次重建(禁止 journal 磁带当 live 回放)、`session/resources` 下行 MUST NOT 要求 bump 协议版本(r1790)。

## 2. 版本兼容现状

- `PROTOCOL_VERSION: u32 = 2`(envelope.rs:9),`host.describe` 返回;客户端硬等值校验,不匹配即致命、无降级无重试风暴(协议版本硬闸 ath44)。
- attach 预检另比对 `/healthz` 的包版本(attach.rs:110-139)。
- 演进策略 = 「宽容解码 + 不 bump」:Event 字段大量 `#[serde(default)]` 容忍旧发送方;未知下行方法可忽略。**无能力协商、无 per-method 版本、无 schema ID**。
- 推论:多端分发后(手机 app 发版慢于 daemon),硬等值会逼成全端同步升级——这是 v3 引入字段级演进的核心动机。

## 3. 强/动态分界线(fdl 管辖边界的依据)

**信封层全动态**:OpenAPI 对 `params` 仅一句 `"method params; shape per product method table"`(oapi.rs:94),且刻意锁死 envelope-level(oapi.rs 测试断言 per-method schema 会成为第二词表;oapi.rs:4 注释:payload 形状 SSOT 是产品方法表不是文档)。

**载荷层强类型(Rust 内存)**:registry `parse_command`(registry.rs:470-479)tag-injection 后 serde 解码为 38 变体具体 enum;Event 18 变体同理。

**显式动态字段清单**(= 未来 fdl 里的 `string` 原文区,分界线与 serde 类型签名一一对应):

| 字段 | 类型 | 性质 |
|---|---|---|
| `ToolStart.args` | `serde_json::Value`(event.rs:27) | 工具参数,形状由各工具运行时 JSON Schema 决定 |
| `MessageStart/End.message` | `Option<Value>`(event.rs:55/60) | **领域层本强类型**,`to_wire_event` 主动 `to_value` 擦掉(event.rs:130-141);`AgentMessage` untagged 靠形状试探 |
| `ToolEnd.result` | `String`(event.rs:32) | 工具输出**原文内嵌**——即 fdl 动态块推荐模式的现成先例 |
| `ClientRequest.payload` / `RpcResult.value` / resources snapshot | `Value`(envelope.rs) | 通用动态载荷 |

推论:**fory 化不移动这条线**,只是要求把它从 Rust 类型签名升格为 fdl 显式声明;且 `MessageStart.message` 这类被擦类型的半动态块,反而能借 fdl union 的 case_id 判别升级为强 schema(net win,见 03 §3.3)。

## 4. 大载荷与吞吐机制(二进制化收益区)

- journal 环形 10k 条、冷恢复一次性重放上限 10,000 条;`MUX_CHAN_CAP=16_384` 定容依据即此。
- 下行合并窗口 10ms(合并窗口 ath43,`src/app/core/remote.rs` coalesce,BDD 锁行为)。
- 大载荷过线全是 JSON:get_messages 全量 transcript(SESSION_VERSION=7)、session_tree 全树、ToolEnd 全文、含 base64 图片的 AiBridgeMessage。
- **现状缺口**:全项目没有线协议序列化 benchmark——v3 决策前应先补,拿 JSON baseline。

## 5. 接缝(改动收敛点)

1. `HostClient` trait(`host_client/mod.rs:44-70`):二进制化只动 `HttpWsClient` 的 encode/decode 路径;`InProcessClient` 与 Print/嵌入路径零影响。
2. `codec.rs`:「product wire bytes SSOT」落点正确,但需先收拢旁路(服务端 unary 应答现走 jsonrpsee `raw_json_request` + `polish_rpc_json`,下行走 `jsonrpc_notification(...).to_string()`,客户端上行走 `jsonrpc_request`)。
3. `registry::parse_command`:上行方法名 → Command 的单点,可替换为编译期方法 ID + 二进制 union 解码;38 行 `MethodEntry` 能力位表(Auth/Idem/Resp/Exec)与编码无关,可平移。
4. `XyEvent ↔ Event` 映射层(event.rs:114-241):今天为 JSON 形状差异付的双重转换税,v3 可合并为一层。

## 6. 被动面(切换时会被牵动)

1. **规格合约**:server-core.feature r1778/r1796(产品路径必须 JSON-RPC 2.0)、r1803/r1804(WS 帧必须 JSON-RPC、拒非 JSON-RPC 应用帧)+ `src/AGENTS.md` 信封 normative 行——须走 propose 改产品级规格。
2. **jsonrpsee 依赖失效**:rpc_module.rs 整个分发建立在 JSON 字符串上;v3 = 自建二进制分发(方法表平移)。
3. **WS 二进制帧今日为致命**:服务端 mux `!msg.is_text()` 即断连(http.rs recv_loop);客户端只处理 `Message::Text`。双轨期需帧类型分派。
4. **BDD/夹具重写**:steps_server.rs 解析 JSON body;`EchoHost`/`ScriptedMuxHost`(in_process.rs:28-70)直接构造 `RpcMessage` 对象;codec/event/envelope/registry 数十处 JSON 字段名断言。
5. **调试面**:curl / `/openapi.json` / `/docs` 全建立在 JSON 上——v3 用 xlang 自描述性补偿(帧 dump 工具),openapi 保留为方法表文档。
6. **不受影响**:journal 存储(内存对象)、幂等账本、writer 租约(带外 header)、`/healthz` 与 attach 预检。
