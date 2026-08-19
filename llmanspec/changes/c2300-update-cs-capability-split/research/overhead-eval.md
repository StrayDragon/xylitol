# 载体开销评估：统一 CS（A）vs 单进程自持（B）

> 目标：给 `c2300-update-cs-capability-split`「启动与连接方式」的 A/B 候选提供决策依据。
> 结论以**一扇默认 TUI 可用性**为准，不追理论最优。分析模型 + 门槛先行，数字待 lab 实测（方案见下）；尚未测得结论前 A/B 保持 Open。

## 决策问题

- **A（统一 CS）**：默认也跨进程走网络（同机 = loopback），代码路径唯一、SSOT 风险最低，但单窗恒多一个 host 进程。
- **B（单进程自持）**：默认与 host 同进程直调（今天形态），进程税低；代价是多一条"进程内漏斗"，若与 A 语义分叉则违背 SSOT。
- 关键前提：A/B 共享**同一契约**（Command/Event）；差异只在**载体**——socket + 序列化 vs 进程内直传。契约唯一不因 A/B 改变（c2300 已定）。

## 热路径分解（A vs B 的差异点）

```text
XyEvent → to_wire_event → serde → [socket 帧 / 进程内 channel] → 反序列化 → N 个订阅者
```

| 成本项 | B（进程内直传） | A（本机网络） | 是否主导 |
|---|---|---|---|
| serde 序列化/反序列化 | 可免（直传结构体）；但要免就得走"结构体路径"，与 A 语义一致性靠符合性闸守 | 必付（每帧一次 tagged JSON 往返） | **共同项**，A 必付 |
| WS 帧头 / mask | 无 | 每帧 ~十几字节头 + client→server mask | 小 |
| 传输 | tokio channel（进程内，无内核往返） | loopback TCP / UDS（每帧少量 syscall） | 小 |
| 广播到 N 窗 | 有（journal Arc 字节 / 多订阅者 clone） | 有（host 侧向每连接 broadcast） | **共同主导项** |
| 每帧分配 | 有（TextDelta 的 Vec/String） | 有 | **共同主导项** |

## 现实量级（估，待测）

- token 率：单会话典型 **20–100 tok/s**，快模型数百；极端按 ~500–2000 帧/s（TextDelta 粒度）上界。
- serde 小对象：约 1–几 µs/帧 → 1000 帧/s ≈ 个位数 ms/s，**<1% 单核**。
- loopback 往返：~几十 µs，**远小于 LLM 首 token（~百 ms 级）**——延迟可忽略。
- 推断假设（待实测确认）：**传输/socket 不是主导**；主导是 serde + 分配 + 广播。这与 c2280 research `05` 吞吐分层一致（进程切分 > 编码 > 成帧 > 传输 > runtime）。

## 决策门槛（gate，实测后判定）

- A ≥ B 不合格的信号：在 **500 tok/s** 流下，A 相对 B 的 TUI 单窗 **CPU 增量 > ~5%**，或 **p99 帧耗**超过显示/输入节拍（约 >16ms）造成肉眼卡顿。
- 未超门槛 → **判 A 可作默认**（网络载体打开，代码唯一）。
- 超门槛（或 dense/多窗放大）→ **默认 B（进程内直传）**，`--attach` 仍是唯一网络路径；此时更该优化的不是换编码，而是广播少 clone / 少分配。
- 附带判据：A 恒多一个 host 进程对**资源受限启动体感**的影响（单窗场景的进程税）——若体感差，即便传输开销不超标，也可能默认 B。

## 测量计划（lab spike；复用现有 criterion bench）

- **fixture**：模拟 LLM 流——100 / 500 / 2000 帧/s × 帧长 1–40 字符的 `TextDelta` 序列。
- **三路载体同机对照**（同一 fixture）：
  ① 进程内 channel，免序列化（B 直传上界）
  ② 进程内 channel，tagged JSON 序列化（B 带契约/ A 上界）
  ③ loopback TCP（或 UDS）+ WebSocket 帧，tagged JSON（A 实况）
- **指标**：每帧 p50 / p95 / p99 耗时、总墙钟、单核 CPU%、分配次数与字节、N=2/4 窗广播放大。
- **输出**：三路对照表 → 套入 gate。

## 对 c2301（稳定线协议）的影响（结论先行）

- A/B 不改变协议闭集工作（契约唯一，照做）。
- 若 gate 支持 A：**不必为"省网络"预埋二进制 codec**；tagged JSON 为主，首要优化是 journal 广播少 clone（`05` 编码层）。
- 若 gate 拒不：默认 B（进程内直传）+ attach 网络路径；协议仍 tagged JSON，二进制 codec（postcard / rkyv）留到「确实受分配/序列化主导」时再开。

## 实测结果与结论（lab/carrier_bench，c2300/lab，一次性）

N=50_000 帧、text 1–40 字符、tagged JSON `{type,text}`；三路同机，`cargo run --release`（单次运行；余量以数量级计，不另做统计）。

| carrier | total wall | capacity | avg/frame | p50 µs | p95 µs | p99 µs |
|---|---|---|---|---|---|---|
| channel-raw（B 上界，无 serde） | 14.28 ms | 3.50M fps | 0.03 | 0.03 | 0.04 | 0.05 |
| channel-json（进程内 + serde） | 25.19 ms | 1.99M fps | 0.18 | 0.18 | 0.24 | 0.27 |
| tcp-json（loopback + 长度前缀 JSON） | 22.17 ms | 2.26M fps | 0.11 | 0.10 | 0.16 | 0.18 |
| bcast4-json（channel-json 扇出 4 消费者） | 155.66 ms | 321k fps | 0.23 | 0.20 | 0.39 | 0.48 |

判读（对照 gate：500 tok/s、p99 < 16ms）：

- 一切载体容量 ≥ ~320k fps（≥ 门槛 **640 倍**），p99 ≤ 0.5 µs（≤ 16ms 的 ~3 万倍）。**余量数量级。**
- serde 代价：channel-raw → channel-json 每帧 ~+0.15 µs（约 5×），绝对值微。
- 传输代价：channel-json vs tcp-json 同量级（tcp 名义略好属并发/分配噪声，**不**解释为"网络更快"）；**网络/传输非主导**成立。
- 广播扇出 4 是测试中最大的相对项（~6×），印证「广播/多窗 clone」是共同主导里的大头；即便如此仍 640 倍于门槛。

结论：

1. **开销不构成反对 A 的理由** → 默认取 **A（统一 CS / 网络载体）**，代码路径唯一、SSOT 最稳。
2. B（单进程自持）不再由性能支撑；保留与否只看**单窗进程税与启动体感**（产品体验判定，非性能）。
3. 对 c2301：**tagged JSON 作主编码足够**（µs 级、百万级 fps）；**不预埋二进制 codec**；首要优化顺位是广播少 clone / 少分配（即便不优化余量也巨大）；postcard / rkyv 留到极端 dense / 多窗再开。

测量切口（诚实）：纯管道饱和微基准（多核并行 wall），非整机 / 单核逐帧；未含 WS 帧头 / mask（以长度前缀代替，固定小头）；未实测 500 tok/s 定速 CPU。量级差距使以上不影响结论。复现：`cd llmanspec/changes/c2300-update-cs-capability-split/lab/carrier_bench && CARGO_TARGET_DIR=/tmp/cb-target cargo run --release`。

## 参照

- c2280 research `01`（整体框架选型与热路径）、`05`（吞吐分层、编码选项、io_uring 适用面）。
- 本仓已有 criterion bench 基建：`benches/`（`benches/token_estimator.rs`）、`Cargo.toml` `[dev-dependencies] criterion = 0.8`。
- LLM streaming 典型速率（Anthropic / OpenAI 流式常识量级）。
