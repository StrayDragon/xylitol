# OTEL 与 Langfuse 观测

> **首要可观测出口**：进程内 fastrace 时间线 → 可选 **OpenTelemetry (OTLP/HTTP)** → **Langfuse**（或任意 OTLP 后端）。
> **不**自研 Inspect 检视台 / MITM 作主路径。
> 现状对齐：2026-07-22。
> 本地对照底座已在 architecture；本文只留「远程/标准时间线出口」候补。

## 用户怎么碰到

排障或复盘时：打开 Langfuse（或 Jaeger 等）→ 按 session / turn 看 agent 过程与模型调用 → 需要原始 SSE 对照时仍用本地 `provider-trace.jsonl` + `just obs-*`。

## 范围

| 层级 | 内容 | 承诺 |
|---|---|---|
| **A · 进程内时间线** | fastrace span（provider / ReAct / tool 等） | 已有底座；出口可选 |
| **B · OTLP 出口** | OTLP/HTTP 导出（feature `otel` + 配置显式开启） | **首刀**：默认不收集 |
| **C · Langfuse 语义** | GenAI / `langfuse.*` 属性、generation / tool 观测类型 | **后置**切片 |
| **非目标** | 自研 Web Inspect 台、全球 MITM、默认明文甩敏感全文 | 不做 |

## 产品规则

| MUST | 禁止 |
|---|---|
| 默认 `exporter=none` / 未配置 → 零 OTLP 流量 | 默认把完整请求体永久明文甩到不可控远端 |
| 坏配置 / 后端未就绪 → 降级不收集，进程可继续 | 因观测装配失败而阻断对话主路径 |
| 时间线栈保持 fastrace；OTLP 经 fastrace-opentelemetry | 引入 `tracing` 双栈 |
| 敏感载荷仍跟本地闸（显式 debug / provider-trace） | 把「开了 OTEL」等同「全文 raw 默认上传」 |

## 分阶段（可认领切片）

| 阶段 | 用户可感知结果 | 对应 change（意向） |
|---|---|---|
| **M1 OTLP 装配 + 默认 none** | 配置开启才导出；未配/坏配不收集 | `c1475-add-otel-export` |
| **M2 GenAI / Langfuse 属性** | Langfuse 里可读 generation、tool、session、用量 | `c1480-add-otel-genai-langfuse` |
| **M3 载荷档位（可选）** | 显式档才带截断/全文 observation input-output | 后置；默认可只元数据 |
| **M4 子进程出站（可选）** | 托管 bash/MCP 对外请求策略透明 | 后置；不假装已全捕获 |

## 与本地 JSONL 的关系

| 产物 | 用途 |
|---|---|
| `provider-trace.jsonl` + `just obs-*` | 离线 / 窄读 / raw↔mapped 对照（线路 vs 产品理解） |
| OTLP → Langfuse | 过程树、session、模型与工具状态的人机友好 UI |

二者同源（同一 fastrace 事实），**不**互替本地对照底座。

## 依赖与并行

- **硬依赖**：进程内 fastrace 对照底座（已落地）。
- **软依赖**：本机或远程 Langfuse / Collector（用户自备；未就绪时 M1 仍可 `exporter=none` 验收装配）。
- 可与 Tokenizer / 多模态 / TUI 视觉并行。

## BDD 意图示例

**场景：默认不出口**
Given 用户未配置 `[otel]` 或 `exporter = "none"`
When 正常对话产生 fastrace span
Then 不向任何 OTLP 端点发送流量

**场景：坏端点降级**
Given 用户配置了 OTLP 但端点不可达或凭证缺失
When 进程启动并跑一轮对话
Then 观测降级为不收集（或仅本地 JSONL），对话主路径不失败

## 相关

- 现行对照底座：[../architecture/进程内观测.md](../architecture/进程内观测.md)
- 总索引：[README.md](./README.md)
