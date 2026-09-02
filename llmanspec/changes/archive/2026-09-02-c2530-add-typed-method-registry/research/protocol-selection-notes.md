# 协议选型分析笔记（2026-09-02 会话结论）

> 本笔记固化 2026-09-02 关于 TUI↔Host 传输形态的多轮选型分析结论，作为本 change 的背景依据。
> 详细论证过程见当日会话；此处只留结论与证据指针。

## 宪法：单一 wire 协议

- **唯一 wire = 四象限信封 over `POST /api/{method}` unary + WS mux（JSON）**。新客户端一律实现同一信封，禁止发明第二种 framing。
- 信封载体无关：未来若收敛为单条全双工 WS 是机械迁移，语义零改动——这个开闭性质必须守住。

## 已评估并否决的替代项（含理由）

| 替代项 | 否决理由 |
|---|---|
| gRPC/tonic | SSOT 反转（proto 成真值，违背「类型 SSOT 在 Rust」）；浏览器需 grpc-web 代理；二进制 wire 人眼不可读；对 loopback 流量性能零可测收益；唯一优势（跨语言互操作）对一方 Rust 客户端集不成立 |
| 单条全双工 WS | 命令通道与流健康解耦（流卡死时 steer/abort 仍可达）是硬价值；丢失 curl 调试；鉴权要走首帧 |
| stdio/UDS JSON-RPC（ACP 形态） | 杀死 daemon + 多客户端 attach 模型（注册发现、写者租约） |
| MCP Streamable HTTP | per-request 流与「会话级流 + 多 attach」模型相逆 |
| JSON-RPC 2.0 | 类型化四象限比 notification 松语义强，无增益 |
| ConnectRPC | Rust 侧生态不成熟 |
| 二进制信封（MessagePack/CBOR） | 省 30% 体积 vs 丢 curl 可调试性，loopback 个人工具必亏 |

## 关键量级证据

- 性能瓶颈在消费侧投影不在传输：c2490 基线（release）冷恢复 15 万行 9.96s、内存 1.42MB/千行；交互重绘 p50 5–25µs 达标。传输 delta 每帧解析 µs 级，比投影低四个数量级。
- prompt 已经是异步作业（`host.rs` prompt handler `tokio::spawn` 后立即 ack，事件走 mux）；同步残留仅 bash（30s 闸）/reload（300s 闸）。
- 可观测性：进程内层（provider-trace JSONL、journal、obs 管线）与协议无关；线级层双载体的「命令通/流死」天然二分是排查时的真实资产。

## 外部参照

- **dsh（deepseek-harness）**：同核心多载体（web HTTP+WS / SDK stdio JSON-RPC / ACP）；Typert = typed registry 工业实现（build-time codegen + 严格校验 + Zod schema）；carrier generation 重连模型；cookie 过 WS 升级鉴权。参照不对标（产品定位不同）。
- 仓库内延后票：c2480（重连状态机）、c2485（凭据门禁）、c2525（投影增量化）与本结论的关系见各自 proposal。

## 本 change 与结论的关系

typed registry 是上述结论中「协议不变、仓库内施工」的核心项：把方法接缝从字符串收敛为类型化声明，是未来 gpui / Rust-wasm / TS 客户端接入成本的共同下界。tonic 对照探针报告：`lab-tonic-probe-report.md`（待补）。
