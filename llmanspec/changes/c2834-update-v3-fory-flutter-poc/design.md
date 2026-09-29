# Design: 线协议 v3(fory 二进制信封)

> 证据与完整论证见 `research/`(README 导览);本文只固化决策与实施约束。
> 基线:xylitol `main@f4b8d68e`;fory `963cb37`(锁 release 1.7.5)。

## 1. 目标与非目标

**目标**:产品线协议从 JSON-RPC 2.0 迁移到 schema 化二进制信封(fory xlang 编码,fdl 单源生成 Rust/Dart),支撑 Flutter 多端;字段级版本演进替代 `PROTOCOL_VERSION` 硬等值;协议类型单源(IDL)替代「Rust 类型 + registry 测试锁」。

**非目标**:不动领域层(`XyEvent`/`AgentMessage`/`SessionEntry` 持久化格式,SESSION_VERSION 不变);不动 InProcess 零序列化路径;不引入 gRPC/tonic(flutter web 代理税 + 自定义下行语义失真,见 research/04 §3);不迁移 journal 存储。

## 2. 关键决策

| # | 决策 | 依据 |
|---|---|---|
| D1 | 动态载荷(工具参数等任意 JSON)= `string` 原文过线 | fdl `any` 不支持 list/map/数字且须两端注册(research/03 §3.1);`ToolEnd.result` 现成先例 |
| D2 | 信封 `Frame = ClientRequest{rpc_id, method_id, command} \| ServerResponse \| ServerNotification{seq, event}`;方法名不过线,`method_id` 编译期分配 | research/04 §2;registry `MethodEntry` 平移为 ID 表 |
| D3 | 编码载体 = fory xlang compatible 模式(非 same-schema) | 字段级前后向兼容是核心动机;same-schema 等价又一个硬等值 |
| D4 | server framework 保留 salvo;jsonrpsee 在硬切阶段退役 | fory 化不要求换框架;接缝在 codec 层(research/04 §3) |
| D5 | fork foryc(基于 fory@963cb37)加 `--rust-serde`:生成物同时带 serde derive | 双轨对拍零转换层 + 持久化层类型复用;f-string 模板 fork 薄(research/03 §3.5);同步向上游提 PR |
| D6 | 字段保留字改名 11 处(3 词:`message`×8/`list`×1/`timestamp`×2),只落 fdl 与 wire 映射层 | research/04 §2.5;领域层/持久化零改名;对拍点设领域对象层消除名字税 |
| D7 | 双轨 = 验证手段非共存常态:双轨期旧 JSON-RPC 路径 MUST 保活(对拍与回退保障),对拍全绿后**硬切**,旧 spec 条款与 jsonrpsee 绑定**延后**到硬切任务一并移除 | 用户指示;单人工具同步发版,无线上长期双轨负担 |
| D8 | 版本协商:`host.describe` 声明 wire formats;v3 客户端不识别则致命断开(不降级、不重试风暴,平移 ath44 语义) | research/04 §4 |

## 3. 信封与寻址

- **上行** `ClientRequest{ rpc_id: uint64, method_id: uint32, command: Command }`;`rpc_id` 承接幂等键(语义同 r1781 的 JSON-RPC id)。
- **下行** `ServerNotification{ seq: uint64, event: Event }`;`seq`/journal/resync/订阅语义(r1806/r1798/r1807/r1808/r1792)格式无关,原样复用。固定区 `session/resources`、审批/问卷告知在 v3 下为对应 Event/通知变体,「不消耗 seq、不进 journal、旧端可忽略」语义保持(r1790)。
- **写者租约**:HTTP header `X-Writer-Token` 载体不变;WS v3 帧内等价成员(对齐今日 WS 顶层 `writerToken` 的连接本地租约语义,r1793)。
- **未登记 `method_id`**:稳定失败,产品码等价今日 -32601 语义(r1713 对齐)。

## 4. 双轨与切换路径

```
阶段 1 codegen 基建 → 阶段 2 服务端双轨 → 阶段 3 客户端双轨
→ 阶段 4 对拍全绿(领域等价断言)→ 阶段 5 硬切与清理(延后处理)
```

- 双轨期同一端口同时服务:WS text 帧 → 既有 JSON-RPC 路径(不动);WS binary 帧 → v3 路径。POST /rpc 按 content-type/载荷形态分派。
- **对拍纪律**:对拍点设在领域对象层(`XyEvent`/`Command` 语义),JSON 路径与 v3 路径各自解码后比较,不经字段名映射;未通过对拍的能力 MUST NOT 仅存在于 v3 路径。
- **延后移除清单**(阶段 5,依赖阶段 4 全绿):spec 旧条款措辞改写(server-core r1778/r1796/r1803/r1804/r1809、protocol-app r1701/r1709/r1696 等 JSON-RPC 钉死处)、jsonrpsee 依赖、WS 文本帧产品路径、四象限 envelope 旧类型、`PROTOCOL_VERSION` bump。
- 旧客户端共存:不承诺(v3 硬切后旧客户端被版本闸挡,attach 预检提示升级;单人工具同步发版)。

## 5. 演进与版本

- fdl 字段编号永不复用,删除字段 `reserved`;新增字段/变体不要求 bump 版本(未知变体 → `UnknownCase` 载体,可降级忽略,对齐 r1719/r1790 精神)。
- fory 锁 release 1.7.5(Rust crates.io 与 Dart pub.dev 同号);两端同 commit。conformance 字节快照测试防上游 codec 漂移。
- 「协议支持矩阵」心智:格式稳定期内各端字段演进独立;fory 运行时升级需全端协调。

## 6. 测试策略(seam)

**Seam(复用既有 harness,不发明新缝)**:

1. **BDD 行为合约**:`tests/bdd/steps_server.rs` / `steps_protocol.rs` / `steps_remote_resilience.rs`(起真实服务端 → 发帧 → 断言);v3 新场景新增 step:binary 帧发送探针、协商断言、对拍断言。
2. **编解码单测**:生成物 roundtrip、未知字段/变体跳过、动态块原文保真(r1720 等价)、字节 conformance 快照。
3. **对拍测试**:双路径领域等价(事件流、幂等回放、租约冲突、审批 first-wins)。
4. **benchmark**:JSON baseline → v3 对比(criterion 或统计测试,对齐 `token_estimator_bench.rs` 形态)。

## 7. 工具链与仓库落点

- fdl 真源:`src/protocol/wire/v3/xy_wire_v3.fdl`(协议切片 PoC 产物升级为全量);生成物 check-in(可 diff 审查),`just codegen-wire` recipe 用 vendored foryc 重新生成并 diff 校验(CI 防漂移)。
- fork foryc 落点:`tools/foryc/`(基于 fory@963cb37 的最小 patch:`--rust-serde`);同步上游 PR,合入后切回官方。
- Dart 侧:Flutter 工程接入时 `build_runner` 生成 codec part(research/03 §3.4);CI 增可选 job。

## 8. 风险与退路

| 风险 | 缓解 |
|---|---|
| fory 上游 breaking / codec 漂移 | 锁 1.7.5 + conformance 快照 + fork vendor 意向 |
| 双轨期复杂度(两条路径两套测试) | 阶段 5 硬切移除清单一次性收口;对拍纪律保证不产生「仅 v3 可用」能力 |
| fdl 全量后发现表达力缺口(untagged `AgentMessage` 细节) | 阶段 1.1 首个任务即全量草稿,缺口在基建期暴露;protobuf 为退路(D2 寻址/信封设计格式无关) |
