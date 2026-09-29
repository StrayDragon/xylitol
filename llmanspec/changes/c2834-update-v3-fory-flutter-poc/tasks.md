# Tasks: c2834-update-v3-fory-flutter-poc

> 任务依赖:阶段 1 → 2/3(可并行)→ 4 → 5。阶段 5 为**延后处理**(对拍全绿后
> 且经确认触发),对应「旧 spec 条款与 jsonrpsee 绑定等移除延后,保对拍」策略
> (design §4)。测试 seam 见 design §6(复用既有 BDD harness)。

## 0. 前置门

- [ ] 0.1 复核 research/ 结论与 fory 锁定版本(1.7.5;crates.io 与 pub.dev 同号);
      若上游已出新的稳定 release,更新 research 基线快照后再开工。
- [ ] 0.2 确认 fork foryc 落点(`tools/foryc/`,design §7)与 `--rust-serde`
      patch 范围;向上游提交 PR 的意向记录进 research。

## 1. codegen 基建

- [ ] 1.1 全量协议 fdl:`src/protocol/wire/v3/xy_wire_v3.fdl` 覆盖全部
      Command(38)/Event(18)变体、Frame 信封、session/model 过线载荷;
      落地字段改名表(research/04 §2.5:11 处)与 `reserved` 编号纪律;
      表达力缺口(untagged AgentMessage 细节等)在此暴露并记录。
- [ ] 1.2 fork foryc(基于 fory@963cb37)加 `--rust-serde`,vendored 到
      `tools/foryc/`;生成物同时携带 fory 与 serde derive。
- [ ] 1.3 `just codegen-wire` recipe:fdl → Rust 生成物 check-in +
      re-generate diff 校验(防手改漂移);方法 ID 表与 registry
      `MethodEntry` 对齐测试(ID ↔ 方法语义一致)。
- [ ] 1.4 字节 conformance 快照测试:代表性帧(含动态块原文、未知变体、
      optional 缺省)锁 bytes,防 fory 上游 codec 漂移。
- [ ] 1.5 线协议 benchmark:JSON 路径 baseline(事件批量、transcript 快照、
      ToolEnd 大载荷三形态),为 v3 对比提供基线数据。

## 2. 服务端双轨数据面

- [ ] 2.1 WS 帧分派:binary → v3 解码路径,text → 既有 JSON-RPC 路径原样保活;
      非法 binary 帧按致命处理(对齐今日非法文本帧语义)。
- [ ] 2.2 POST /rpc 二进制 body 支持(content-type 分派),JSON body 路径不动。
- [ ] 2.3 `host.describe` 声明 wire formats 能力;v3 客户端协商失败 →
      致命断开(不降级、不重试风暴,平移协议版本硬闸语义)。
- [ ] 2.4 上行 v3:`method_id` 解码 → 等价 registry 路径 → 复用 dispatch
      (r1776 共享执行语义不破);未登记 `method_id` 稳定失败(对齐 -32601)。
- [ ] 2.5 下行 v3:Event → v3 编码 + seq/journal/resync 复用;
      `session/resources`、审批/问卷告知的 v3 变体(不消耗 seq、不进 journal、
      可忽略语义保持)。
- [ ] 2.6 v3 通路写者租约与幂等:rpc_id 幂等键接入既有账本(语义同 r1781);
      WS 连接本地租约等价载体(语义同 r1793)。

## 3. 客户端双轨

- [ ] 3.1 `HttpWsClient` 双帧编解码:协商选择 v3,能力缺失回退 JSON-RPC;
      InProcess 路径零改动。
- [ ] 3.2 `XyEvent ↔ v3 Event` 映射层(对拍点 = 领域对象,无字段名映射表)。
- [ ] 3.3 TUI attach 路径在双轨期保持默认 JSON-RPC(v3 经实验开关启用)。

## 4. 对拍与门禁

- [ ] 4.1 BDD step 扩展:binary 帧发送/接收探针、协商断言、双路径对拍断言
      (复用 steps_server/steps_remote_resilience 既有服务端启动模式)。
- [ ] 4.2 对拍测试:同行为(事件流、幂等回放、租约冲突、审批 first-wins、
      冷恢复快照)经双路径产生领域等价结果;对拍未过的能力不进 v3-only。
- [ ] 4.3 Dart 解码 smoke:research/poc/dart_demo 升级为面向生成物的冒烟
      (CI 可选 job),验证 Rust 编码 → Dart 解码链路持续成立。

## 5. 硬切与清理(延后处理;依赖阶段 4 全绿后确认触发)

- [ ] 5.1 TUI attach 默认切换 v3;JSON-RPC 路径降级为纯调试用途。
- [ ] 5.2 spec 旧条款改写:server-core r1778/r1796/r1803/r1804/r1809、
      protocol-app r1701/r1709/r1696 等 JSON-RPC 钉死措辞更新为 v3 终态
      (v3 新条款在落地 specs 时已写入,此处只移除迁移期措辞)。
- [ ] 5.3 退役 jsonrpsee 依赖、WS 文本帧产品路径、四象限 envelope 旧类型;
      `/openapi.json`、`/docs` 措辞更新(保留方法表文档职能);
      `PROTOCOL_VERSION` bump。
- [ ] 5.4 帧 dump 调试工具:二进制帧解为可读 JSON(xlang 自描述),补偿
      curl 调试面损失。
