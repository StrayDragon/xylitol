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
| D5 | **不 fork(修订:原方案为 fork foryc 加 `--rust-serde`)**;直接用现有 compiler | 对拍经 3.2 映射层汇领域对象(D8 对拍点),持久化不进 schema(wire 只管投影),夹具断言用生成物自带的 Debug/Clone/PartialEq——fork 的三个原始动机全部被后续决策消解;仅当实施中确需 serde 形态(如 JSON dump)再做薄 fork,晚做不亏 |
| D6 | **协议真源用 fbs(FlatBuffers schema)前端,字段名零改名**(修订:原方案为 fdl + 11 处改名) | fdl 三条逃逸路实测全败、fbs 前端全通(research/05);wire 字段名与现有 JSON 线逐字段一致;代价:类型 ID auto-hash、字段编号=声明顺序(尾部追加纪律)、map 用 keyed vector(仅 1 字段) |
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
4. **benchmark**：JSON baseline → v3 对比（落为统计测试 `size_baseline_vs_json_large_transcript`）。两个比例分属不同通路：0.73 = 强 schema union（`MessagesResult`，即 2.5b 接完后的形态）；≈ 1.0006 = 当前产品通路（非 describe 应答一律 `RawOk`，JSON 原文入 string，100KB 仅 +64B 信封开销）。引用体积叙事时得带上通路。

## 7. 工具链与仓库落点

- 真源：`src/protocol/wire/v3/xy_wire_v3.fbs`（fbs 前端，D6 修订后不再用 fdl 后缀；协议切片 PoC 产物升级为全量）；生成物 `generated.rs` check-in（可 diff 审查）。`just codegen-wire` = `scripts/gen_wire_v3.py`，经 `PYTHONPATH=<../fory>/compiler` 直调 fory 仓内纯 Python compiler（无 `tools/foryc/` vendored 目录，D5 修订已取消 fork），`--check` 做字节级 diff 校验。
- 无 fory clone 时的必跑探针：BDD `codegen-diff-clean` 以「fbs 声明类型集 ↔ `generated.rs` 的 `pub struct`/`pub enum` 集」1:1 兜住结构漂移（字节级对拍仍靠有 clone 的机器跑 `--check`）。
- Dart 侧:Flutter 工程接入时 `build_runner` 生成 codec part(research/03 §3.4);CI 增可选 job。

## 8. 实施编排(多 agent 并行规划)

### 8.1 依赖结构与并行点

串行依赖点(不可并行):**1.1/1.3**(全量真源与生成物是所有后续任务的公共地基;1.1 边写边暴露表达力缺口,可能回改设计)、**4.x 收口**(对拍需要全局视角 + 共享测试基建)、**5.x**(延后触发,收口性质)。

真正的并行收益点只有三处:

| 组 | 内容 | 机制 |
|---|---|---|
| ~~P0~~(已取消) | 阶段 1 纯串行单线(1.1 → 1.3 → …) | fork 决策取消后(D5 修订)无可并行点;1.1 本身就是全局串行点 |
| **P1**(1.3 后,可三开) | 1.4 conformance ∥ 1.5 benchmark ∥ 3.2 映射层 ∥ 4.3 Dart smoke | 均为测试/小任务,互不依赖;单会话顺序做也可,量不大 |
| **P2**(主并行) | **服务端 agent**:{2.1→2.2→2.4→2.5→2.6} ∥ **客户端 agent**:{3.1→3.3}(2.3 协商由服务端 agent 先行) | 文件面不相交(`src/app/server/` vs `src/app/core/`);3.2 映射层已在 P1 完成为共享 API;**双 worktree + 子分支**,见 8.2 |

墙钟收益估算:全串行 ≈ 8 段,P0+P2 并行后 ≈ 5~6 段(省 25%~35%);协调成本主要在 P2 merge。

### 8.2 P2 落地机制(worktree 子分支)

- 从 `sdd/c2834-…` 分叉两个子分支:`sdd/c2834-w-server` / `sdd/c2834-w-client`,各绑独立 worktree;完成后依次合回 change 分支,finalize 仍在 change 分支(change diff 以 merge-base 现算,子分支合入不影响)。
- **每 worktree 必须 `eval "$(just cargo-wt-env)"`,禁止共用 `CARGO_TARGET_DIR`**(仓库硬规则);sccache 可共享。
- 门禁策略:并行期各 agent 只跑自己面的 `cargo test`(定向 nextest filter)+ clippy;**全量 `just qa` 只在 merge 后由主线跑一次**(避免 live-provider 串行闸与 CPU 争抢)。

### 8.3 冲突规避设计(预埋,消除 merge 面)

1. **BDD 挂载预埋**:1.3 顺手在 `src/tests.rs` 预挂空模块 `steps_wire_v3`(v3 步骤全进这个新文件),两个 agent 都不再碰 `steps_server.rs` 与挂载点 → BDD 层零冲突。
2. **生成物冻结**:wire/v3 生成物 check-in 后由 1.3 recipe 独占再生成权;P2 agent 只消费不重生成。
3. **映射层前置**:3.2 在 P1 完成,服务端(2.5 下行)与客户端(3.1 解码)共享同一 API,不各自发明转换。
4. **justfile/Cargo.toml**:若 P2 双方都要加依赖/recipe,约定 server agent 先提交一次 Cargo.toml 变更再开 client agent,或把可预见的依赖在 P1 一次性加齐。

### 8.4 放弃并行的触发条件(收敛回串行)

- 1.1 暴露表达力缺口导致 Frame/信封设计回改(P2 取消,设计稳定前不开双 agent);
- P2 merge 冲突处理超过约半小时(文件面预估失准);
- 任一 agent 的实现触碰对方文件面(编排失守信号)。

## 9. 风险与退路

| 风险 | 缓解 |
|---|---|
| fory 上游 breaking / codec 漂移 | 锁 1.7.5 + conformance 快照 + 无 clone 时的类型集 1:1 探针 |
| 新增依赖拉入上一代传递依赖（`syn` 1.0.109 / `num_enum` 0.5.11 / `toml_edit` 0.19.15 与现用 syn 2.x 并存） | 已接受（8 包纯新增）；bump 时按 skill `xylitol-bump-toolchain` 逐个核，勿因 syn 1.x 而整库 freeze |
| 双轨期复杂度(两条路径两套测试) | 阶段 5 硬切移除清单一次性收口;对拍纪律保证不产生「仅 v3 可用」能力 |
| fdl 全量后发现表达力缺口(untagged `AgentMessage` 细节) | 阶段 1.1 首个任务即全量草稿,缺口在基建期暴露;protobuf 为退路(D2 寻址/信封设计格式无关) |
