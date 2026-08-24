# OTEL 与 Langfuse 观测

> **已落地**底座（本地 JSONL、可选 OTLP、同 turn 过程树、Langfuse 属性、多总线边界）见 [../architecture/进程内观测.md](../architecture/进程内观测.md)。
> 本文只留**未兑现**方向：观测车道分流、门闸压缩语义、运维 Collector、采样与子进程出站。

## 分阶段切片

| 阶段 | 用户可感知结果 | 备注 |
|---|---|---|
| **M-lane** | Langfuse 不再被「门闸失败 / 纯 infra」噪声淹没；可选 Tempo 看失败体验 | `xylitol.obs.lane`；otel19「实际执行」= 过 prepare；直连时应用侧降噪 |
| **M-collector** | 需要 infra 时 endpoint 指 Collector，按 lane 分到 Langfuse / Tempo | 文档 + 示例配置；应用仍单一 OTLP；**不做**应用内双 exporter |
| **M-sample** | 高流量时可尾采样 / 限流而不改业务埋点 | 预留配置意向；默认仍全量（ForceSampled 现状） |
| **M5 子进程出站** | 托管 bash/MCP 对外请求策略透明 | 后置；挂 turn 树或独立 infra lane |

认领时通常一次只提案 **一个** Mn（建议先 M-lane）。

## BDD 意图示例（候选）

**场景：门闸失败不进 Langfuse 语义**
Given 观测开启且 OTLP 直连 Langfuse
When 用户 force compact 且 prepare 报无可摘要历史
Then MUST NOT 出现独立 ERROR `agent.compaction` 根（或不得标为 LLM lane）；面通知仍可经 `XyEvent` / notice

**场景：真·压缩仍在过程树**
Given 观测开启且 turn 内发生过 prepare 的 compaction
When 导出过程树
Then 存在 `agent.compaction` 为该 turn 后代，`xylitol.obs.lane=llm`（落地后），含诚实 `reason`

**场景：Collector 分流**
Given endpoint 指向 Collector 且示例过滤生效
When 同时存在 llm 与 infra span
Then Langfuse 仅见 llm lane；Tempo（或等价）可见 infra

**场景：子进程出站透明（M5）**
Given 操作通过托管 bash 或 MCP 发出外部请求
When 观测开启且配置了 OTLP 出口
Then 该子请求活动与耗时可在过程树或 infra 车道中对照

## 支线与方向

| 支线 | 意向 |
|---|---|
| **Eval 闭环** | 过程树抽样 → Dataset（见 [Agent-Eval与回归基准.md](./Agent-Eval与回归基准.md)） |
| **`xylitol.obs.domain`** | 比 lane 更细的域标签（后置） |
| **OTel Metrics** | 独立管道；禁止用假 span 冒充告警/通知 |
| **敏感载荷档位矩阵** | 调试档 vs 日常档红线字段（文档化） |
| **跨进程 trace 关联** | 子进程 span 与父 `agent.turn` 可关联 |

## 明确不做

- 用观测属性兼做 TUI / 通知总线（`XyEvent` / hooks 已有）
- 应用内双 OTLP exporter / 第二套 `tracing`·OTel span 栈
- 自研 Inspect 检视台作主观测面
- 替用户托管 Tempo/Grafana（仅文档 + 示例）

## 相关

- 现行心智：[../architecture/进程内观测.md](../architecture/进程内观测.md)
- 面事件：[../architecture/用户可见事件.md](../architecture/用户可见事件.md)
- [Agent-Eval与回归基准.md](./Agent-Eval与回归基准.md)
- 总索引：[README.md](./README.md)
