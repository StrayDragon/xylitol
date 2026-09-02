# lab-tonic-probe 报告：tonic/protobuf 打字机流对照探针

> 一次性调研探针（`lab_` 约定，不走 SDD、不入任何闸）。项目位于 `/tmp/lab-tonic-probe`（独立 cargo 工程，未入库，零主仓依赖污染）。
> 目的：把「grpc/tonic 迁移受益不高」从分析感觉升级为实测证据，支撑协议宪法关闭。
> 日期：2026-09-02。

## 环境

| 项 | 值 |
|---|---|
| tonic / prost / tonic-build | 0.13 线 |
| rustc | 1.97.1 |
| protoc | libprotoc 36.0 |
| 机器 | nproc=20，loopback 单进程 server+client（等价产品拓扑） |
| 依赖规模 | 109 个 unique crates（全量依赖树） |
| release 二进制 | 4,272,320 bytes（≈4.3MB，仅 hello-world 级业务） |
| 冷构建 | **14s**（`RUSTC_WRAPPER=` 关 sccache，20 核并行）；sccache 热缓存 4s |

## 量测结果（两轮运行，量级一致）

| 项 | run1 | run2 |
|---|---|---|
| unary 往返 p50 / p95（n=200） | 44µs / 110µs | 71µs / 271µs |
| 打字机流 TTFT（1000 token） | 213µs / 118µs | —（两轮 102–213µs） |
| 逐 token 间隔 p50 / p99 | <1µs / 1µs | 同 |
| 全速流吞吐（顺带测得） | 2,000,000 token / 753ms ≈ **265 万 msg/s** | — |
| 取消传播 | 客户端 drop 流 → 服务端 send-fail 任务干净退出，后续 unary 确认服务端存活 | 同 |

## 解读

1. **gRPC 流式机制对打字机场景确实优秀**：TTFT 百微秒级、逐条送达无合帧、原生取消传播干净。这与先验一致，不是迁移的障碍，也不是迁移的理由。
2. **对比基线不构成收益**：xylitol 现状（POST+WS+JSON）同拓扑下传输层同为 µs 量级，而真实瓶颈在消费侧投影（c2490：冷恢复 15 万行 9.96s，高出本探针全部数字四个数量级）。换 tonic 买不到任何用户可感知的改善。
3. **成本侧确认**：4.3MB 二进制增量、109 crates 依赖面、proto IDL 接管类型 SSOT（违背「类型 SSOT 在 Rust protocol」）、wire 二进制化牺牲线级可观测性、浏览器未来需 grpc-web 代理。全部为持续性税。
4. **取消传播是 tonic 唯一实测亮点**（客户端 drop → 服务端任务干净退出）。xylitol 现状的显式 `abort` 命令 + 合作取消在产品语义上等价（且可跨断线存活），此优势不成立。

## 结论

**维持协议宪法：四象限信封 over POST unary + WS mux，不迁移。** gRPC 的 DX 优点（类型化调用、单点声明）由 c2530 typed registry 在 Rust SSOT 内等价实现；本探针将「是否迁 tonic」议题以实测数据关闭。
