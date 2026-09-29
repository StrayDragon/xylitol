---
depends_on: []
---

# 线协议 v3:fory 二进制信封(Flutter 多端前瞻)

## Why

当前跨进程线协议是自研 JSON-RPC 2.0(信封 `src/protocol/wire/codec.rs` + jsonrpsee 分发 + salvo WS),`PROTOCOL_VERSION` 硬等值无协商。单机 TUI attach 场景下无性能痛点,但产品路线已明确后续用 Flutter(Dart)做 web/desktop/Android/iOS 多端——届时:

1. **版本演进粒度**是刚需:多端发版节奏跟不上 host daemon,硬等值会逼成全端同步升级;fory xlang compatible 模式的字段级演进(未知字段跳过 + `UnknownCase` 载体)正是解耦手段。
2. **类型单源**:现在 38 Command + 18 Event 靠 Rust 类型 + registry 测试锁防漂移(手工 schema);fdl 化后一份 `.fdl` 同时生成 Rust/Dart,新端接入零协议成本,协议 SSOT 从代码上移为声明式 IDL。
3. **大载荷体积/解析**:transcript 快照(journal 重放上限 1 万条)、session tree、含 base64 图片的消息在移动网络下 JSON 编码成本可感知;PoC 实测小消息即省 21%~50%。

**时机判断**:server-TUI(本机 loopback 单客户端)是切换收益最小的场景,现在不切 wire;本 change 先沉淀评估结论与可复现 PoC,切换触发点为 Flutter 端立项或远程场景落地。已完成全链路 PoC:`.fdl` → foryc 生成 Rust(599 行)+ Dart(1132 行)→ Rust roundtrip 冒烟 + **Rust 编码字节被 Dart 原生解码**(详见 `research/`)。

## What Changes

> 草案阶段:以下为方向性清单,落地拆解归 `llman-sdd-propose`。

- 定义 `xy.wire.v3` fdl schema:Event/Command 载荷 union、`Frame` 信封(`rpc_id` 幂等键 + 编译期 `method_id` + 下行 `seq` 保留 journal/resync 语义)、动态块(工具参数等任意 JSON)以 `string` 原文过线(`any` 类型经查不可用:仅支持 bool/string/enum/message/union 动态值且须两端注册)。
- 协议字段名盘点改名(已完成盘点,见 `research/04` §2.5):fdl 保留字撞现有字段共 11 处、3 个词(`message`×8 / `list`×1 / `timestamp`×2),改名只落在 fdl 与 wire 映射层,领域层与持久化零改名;对拍点设领域对象层以消除名字税。
- fork `fory-compiler`(或上游 PR)增加 `--rust-serde` 选项:生成物同时携带 serde derive,支撑双轨对拍与持久化层复用。
- 切换实施(propose 时定):`server-core.feature` 信封合约改写(r1778/r1796/r1803/r1804)、jsonrpsee 退役、WS 文本/二进制双帧分派、`host.describe` 格式能力协商(保留「不匹配即致命」语义)、Dart 侧 build_runner 流程接入 CI。
- 不变项:幂等账本、writer 租约带外通道、InProcess 零序列化路径、journal 存储格式。

## Capabilities

- `server-core`(信封合约 r1778/r1796/r1803/r1804 重写为 fory 二进制信封 + 协商)
- `protocol-app`(Command/Event 词表迁移至 fdl 生成物)

## Impact

- 代码:`src/protocol/wire/`(codec/registry/envelope 重写)、`src/app/server/`(http/ws 分发去 jsonrpsee)、`src/app/core/host_client/`(双帧解码)。
- 依赖:+ `fory`(锁 release 1.7.5,两端同 commit)、+ fork 的 `fory-compiler`;− `jsonrpsee`(切换完成后)。
- 工具链:CI 增加 fdl → Rust/Dart 生成 + Dart build_runner 步骤;新增字节级 conformance 快照测试防上游 codec 漂移。
- 风险:fory compiler 处于 Alpha;锁版本 = daemon 与各端捆绑同一 fory 运行时(格式稳定期内字段演进仍可独立)。退路:protobuf(prost + dart protobuf)同能满足需求,生态更成熟。
