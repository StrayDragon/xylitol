# OTEL 与 Langfuse 观测

> **已落地**底座（本地 JSONL、可选 OTLP、同 turn 过程树、Langfuse 属性、多总线边界、`xylitol.obs.lane=llm` 属性、门禁早退降噪、Collector 示例配置）见 [../architecture/进程内观测.md](../architecture/进程内观测.md)。
> 本文只留**未兑现**方向：infra lane span、运维 Collector 实际分流、采样与子进程出站。分叉身份 / 观测槽 / fork 树边已进 [进程内观测.md](../architecture/进程内观测.md)。

## 分阶段切片

| 阶段 | 用户可感知结果 | 备注 |
|---|---|---|
| **M-lane（余量）** | infra span 也带 lane 标签，Tempo 可按 lane 过滤 | llm 属性与「过 prepare 才进语义 span」已落地（见 architecture 表）；候选只剩 infra lane |
| **M-collector（余量）** | 需要 infra 时 endpoint 指 Collector，按 lane 分到 Langfuse / Tempo | 示例配置已交付（`configs/examples/otel-collector-lane.yaml`）；实际 infra 分流依赖 M-lane 余量；应用仍单一 OTLP；**不做**应用内双 exporter |
| **M-sample** | 高流量时可尾采样 / 限流而不改业务埋点 | 预留配置意向；默认仍全量（ForceSampled 现状） |
| **M5 子进程出站** | 托管 bash/MCP 对外请求策略透明 | 后置；挂 turn 树或独立 infra lane |

认领时通常一次只提案 **一个** Mn。供应商键策略（Zen header 跟树干）park 在 `llmanspec/delayed-changes/models/c2620-add-provider-session-key-policy`。

## BDD 意图示例（候选）

**场景：Collector 分流**
Given endpoint 指向 Collector 且示例过滤生效
When 同时存在 llm 与 infra span
Then Langfuse 仅见 llm lane；Tempo（或等价）可见 infra

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
- 自研 Inspect 检视台作主观测端
- 替用户托管 Tempo/Grafana（仅文档 + 示例）

## 相关

- 现行心智：[../architecture/进程内观测.md](../architecture/进程内观测.md)
- 端事件：[../architecture/用户可见事件.md](../architecture/用户可见事件.md)
- [Agent-Eval与回归基准.md](./Agent-Eval与回归基准.md)
- 总索引：[README.md](./README.md)
