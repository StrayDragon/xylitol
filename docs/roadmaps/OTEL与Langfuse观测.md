# OTEL 与 Langfuse 观测

> **已落地**（M0–M4c：进程内时间线、OTLP 出口、Langfuse GenAI 属性、usage、同 turn 过程树、request JSON + abort）见 [../architecture/进程内观测.md](../architecture/进程内观测.md)。
> 本文只留**未兑现**候补：子进程出站观测。

## 候补切片

| 阶段 | 用户可感知结果 | 备注 |
|---|---|---|
| **M5 子进程出站（可选）** | 托管 bash/MCP 对外请求策略透明 | 后置；不假装已全捕获 |

## BDD 意图示例（候补）

**场景：子进程出站透明**
Given 操作通过托管 bash 或 MCP 发出外部请求
When 观测开启且配置了 OTLP 出口
Then 该子请求的活动与耗时在 Langfuse 过程树中可见（或至少可确认「已发出」「已返回」状态）

## 支线与方向

| 支线 | 意向 |
|---|---|
| **Eval 闭环** | 过程树抽样 → Dataset（见 [Agent-Eval与回归基准.md](./Agent-Eval与回归基准.md)）；本篇不另起检视台 |
| **MCP/bash 出站采样策略** | 高噪声出站可抽样或摘要进过程树，避免爆量 |
| **跨进程 trace 关联** | 子进程 span 与父 `agent.turn` 可关联（排障对照） |
| **敏感载荷档位** | 调试档 vs 日常档的红线字段矩阵（文档化） |

## 相关

- 现行对照底座：[../architecture/进程内观测.md](../architecture/进程内观测.md)
- [Agent-Eval与回归基准.md](./Agent-Eval与回归基准.md)
- 总索引：[README.md](./README.md)
