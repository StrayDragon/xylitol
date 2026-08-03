# Design: c1870 OTel obs.lane + 门闸压缩

## 目标

落地可扩展观测基础的 **M-lane** 切片：发射层诚实标注 `xylitol.obs.lane`，manual compact 与 auto 对齐 prepare-first，直连 Langfuse 时门闸早退不污染 LLM 语义；文档给出 Collector 外部分流示例。

## 多总线（不变式）

| 总线 | 本 change |
|---|---|
| `XyEvent` | 不改闭集语义；compact 面事件仍可发；**不**把 lane 写入事件 |
| hooks | 不改 |
| fastrace | 唯一观测时间线；属性加 `xylitol.obs.lane` |

同现场并列发射（已有模式）：`CompactionStart` +（过闸后）`AgentCompactionSpan`。

## 拓扑

```text
fastrace → Fanout → JSONL
                  → 单一 OTLP/HTTP
                       ├─ 默认直连 Langfuse：应用侧不发射应属 infra 的 LLM 语义 span
                       └─ 可选 → Collector：filter xylitol.obs.lane
                              llm  → Langfuse
                              infra → Tempo/Grafana
```

**锁定：** 应用内不做双 exporter；不做第二套 OTel/`tracing` span 栈。依赖库要 OTel Context 时仅允许 fastrace 桥接 attach。

## `xylitol.obs.lane`

| 值 | 谁带 | 直连 Langfuse |
|---|---|---|
| `llm` | turn / iteration / llm.request / tool.execute / 过 prepare 的 compaction /（结算）token.estimate | 导出 |
| `infra` | 未来 bootstrap/MCP 等；**本切片可不发射** prepare 早退 span | 不建 `agent.compaction` |

弃用名称：`xylitol.signal`（易与 hook/总线混淆）。

## prepare-first（manual）

今日：`CompactionOrchestrator::compact` 先 `CompactionStart` + `AgentCompactionSpan::start`，再 `prepare_compaction` → 早退仍 ERROR span。

目标：

1. 可保留 `XyEvent::CompactionStart`（面）；或 design 实现时若面噪声过大，**最小**改为仅 End/notice——优先保持 Start/End 对称，噪声留给 c1875。
2. **仅** `prepare` Ok 后 `AgentCompactionSpan::start`，`lane=llm`，`reason=manual`。
3. prepare Err：不建 compaction OTLP span；返回错误字符串不变。

Auto threshold/overflow：已 prepare-first，补 `lane=llm`。

## otel19 措辞

「实际执行会话 compaction」= 通过 `prepare_compaction` 之后的压缩尝试（含随后摘要 LLM 失败）。prepare 早退 **不算** 实际执行，MUST NOT 导出 `agent.compaction`（直连路径）。

## Collector 示例落点

- 路径意向：`configs/examples/otel-collector-lane.yaml`（或 `docs/` 短片段链到 example）
- 内容：OTLP receiver → attributes filter on `xylitol.obs.lane` → 两 exporter（Langfuse / Tempo 占位 endpoint）
- 标注：运维可选；不随默认安装启用

## 测试 seam

| Seam | 方式 |
|---|---|
| prepare 早退无 `agent.compaction` SpanRecord | CollectingReporter 单测 |
| 过闸 manual/auto span 含 `xylitol.obs.lane=llm` | 同上 |
| otel19 / lane 合约 | `infra-otel` toon；scenarios `feature: false` |
| 不扩可执行 `.feature` BDD | 与 otel18–21 一致 |

## 非目标

双 exporter、Tempo 部署、c1875 UX、`obs.domain`、采样实现、Metrics、lane→XyEvent、复活 c1490。

## 风险

- 面仍发 CompactionStart 但无 OTel span：排障以 JSONL lifecycle / notice 为准；可接受。
- 漏标 lane：Collector 误路由——主路径 helper 统一打标降低漏网。
