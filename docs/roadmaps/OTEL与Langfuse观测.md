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

## 相关

- 现行对照底座：[../architecture/进程内观测.md](../architecture/进程内观测.md)
- 总索引：[README.md](./README.md)
