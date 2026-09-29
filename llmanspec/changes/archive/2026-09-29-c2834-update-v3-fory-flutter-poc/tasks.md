# Tasks: c2834-update-v3-fory-flutter-poc

> 任务依赖:阶段 1 → 2/3(可并行)→ 4 → 5。阶段 5 为**延后处理**(对拍全绿后
> 且经确认触发),对应「旧 spec 条款与 jsonrpsee 绑定等移除延后,保对拍」策略
> (design §4)。测试 seam 见 design §6(复用既有 BDD harness)。
> 并行编排(多 agent 切分、冲突规避、放弃条件)见 design §8;逐任务依赖以
> `[blocked-by:]` 标记为准。

## 0. 前置门

- [x] 0.1 复核 research/ 结论与 fory 锁定版本(1.7.5;crates.io 与 pub.dev 同号);
      若上游已出新的稳定 release,更新 research 基线快照后再开工。
- [x] 0.2 ~~fork foryc~~(已取消,D5 修订):确认直接使用本地 fory compiler
      (963cb37)即可;`--rust-serde` 仅当实施中确需 serde 形态时再评估薄 fork,
      不作为本 change 前置依赖。

## 1. codegen 基建

- [x] 1.1 [blocked-by: 0.1, 0.2] 全量协议真源(fbs):`src/protocol/wire/v3/xy_wire_v3.fbs` 覆盖全部
      Command(38)/Event(18)变体、Frame 信封、session/model 过线载荷;
      字段名与现有 JSON 线一致零改名(fbs 前端,research/05);编号纪律 =
      声明顺序 + 尾部追加 + 删除保留占位;`thinking_level_map` 以 keyed
      vector 表达;表达力缺口在此暴露并记录。
- [x] 1.3 [blocked-by: 1.1] `just codegen-wire` recipe:fdl → Rust 生成物 check-in +
      re-generate diff 校验(防手改漂移);方法 ID 表与 registry
      `MethodEntry` 对齐测试(ID ↔ 方法语义一致)。
- [x] 1.4 [blocked-by: 1.3] 字节 conformance 快照测试:代表性帧(含动态块原文、未知变体、
      optional 缺省)锁 bytes,防 fory 上游 codec 漂移。
- [x] 1.5 [blocked-by: 1.3] 线协议 benchmark:JSON 路径 baseline(事件批量、transcript 快照、
      ToolEnd 大载荷三形态),为 v3 对比提供基线数据。

## 2. 服务端双轨数据面

- [x] 2.1 [blocked-by: 1.3] WS 帧分派:binary → v3 解码路径,text → 既有 JSON-RPC 路径原样保活;
      非法 binary 帧按致命处理(对齐今日非法文本帧语义)。
- [x] 2.2 [blocked-by: 2.1] POST /rpc 二进制 body 支持(content-type 分派),JSON body 路径不动。
- [x] 2.3 [blocked-by: 1.3] `host.describe` 声明 wire formats 能力;v3 客户端协商失败 →
      致命断开(不降级、不重试风暴,平移协议版本硬闸语义)。
- [x] 2.4 [blocked-by: 2.1] 上行 v3:`method_id` 解码 → 等价 registry 路径 → 复用 dispatch
      (r1776 共享执行语义不破);未登记 `method_id` 稳定失败(对齐 -32601)。
- [x] 2.5 [blocked-by: 2.1, 3.2] 下行 v3:Event → v3 编码 + seq/journal/resync 复用;
      `session/resources`、审批/问卷告知的 v3 变体(不消耗 seq、不进 journal、
      可忽略语义保持)。
- [x] 2.6 [blocked-by: 2.4] v3 通路写者租约与幂等:rpc_id 幂等键接入既有账本(语义同 r1781);
      WS 连接本地租约等价载体(语义同 r1793)。

## 3. 客户端双轨

- [x] 3.1 [blocked-by: 2.3] `HttpWsClient` 双帧编解码:协商选择 v3,能力缺失回退 JSON-RPC;
      InProcess 路径零改动。
- [x] 3.2 [blocked-by: 1.3] `XyEvent ↔ v3 Event` 映射层(注:前置于 2.5/4.x,阶段 1 完成后即做,是服务端/客户端两侧的共享 API)(对拍点 = 领域对象,无字段名映射表)。
- [x] 3.3 [blocked-by: 3.1] TUI attach 路径在双轨期保持默认 JSON-RPC(v3 经实验开关启用)。

## 4. 对拍与门禁

- [x] 4.1 [blocked-by: 2.1, 3.1] BDD step 扩展(新建 `steps_wire_v3.rs`,不动 `steps_server.rs`):binary 帧发送/接收探针、协商断言、双路径对拍断言
      (复用 steps_server/steps_remote_resilience 既有服务端启动模式)。
- [x] 4.2 [blocked-by: 2.6, 3.3, 4.1] 对拍测试:同行为(事件流、幂等回放、租约冲突、审批 first-wins、
      冷恢复快照)经双路径产生领域等价结果;对拍未过的能力不进 v3-only。
- [x] 4.3 [blocked-by: 1.3] Dart 解码 smoke(独立,可早做):research/poc/dart_demo 升级为面向生成物的冒烟
      (CI 可选 job),验证 Rust 编码 → Dart 解码链路持续成立。

## 5. 硬切与清理(延后处理;依赖阶段 4 全绿后确认触发)

- [x] 5.1 [blocked-by: 4.2, 确认触发] TUI attach 默认切换 v3;JSON-RPC 路径降级为纯调试用途。
      (落地:`XyRemoteDriver::new_v3` attach 产品入口显式 v3;v3 连接本地租约已修
      [binary 分支对齐 r1793];库级全局默认翻转留作后继——多 client 租约
      跨连接窗口仍待专项)
