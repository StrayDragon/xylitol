---
change_id: c1490-add-otel-non-agent-backend
title: 非 agent/LLM 时间线的 OTEL 后端选型与分流
status: purpose-draft
priority: 1490
depends_on:
  - c1475-add-otel-export
author: agent
---

# c1490-add-otel-non-agent-backend

## Why

`c1475` / `c1480` 把 **agent / LLM / tool** 观测主路径对准 **Langfuse**（GenAI 数据模型、session、generation）。

但 xylitol 进程里还有大量 **非 LLM** 工作值得看时间线，例如：

- 配置加载 / secret 注入 / bootstrap
- MCP 传输握手、重载、子进程生命周期
- 内置工具侧：bash/MCP 子进程墙钟、文件系统 I/O（非「模型决定」）
- 会话持久化、压缩触发、trust 闸
- 可选 Server / 远程面的 HTTP 请求

这些 span 塞进 Langfuse 会：**污染 LLM 工作区、难做基础设施级过滤/告警、也不符合 Langfuse 产品心智**。需要单独约定「通用 OTEL track」落到什么服务，以及如何与 Langfuse **分流**。

## 推荐方向（设计意向，未实施）

### 首选架构：应用只发 OTLP → Collector 分流

```text
xylitol (fastrace → 单一 OTLP/HTTP)
        │
        ▼
 OpenTelemetry Collector
        ├─ filter: gen_ai.* / langfuse.* / instrumentation xylitol-agent
        │     → Langfuse (/api/public/otel)
        └─ 其余 traces
              → Grafana Tempo（或 Jaeger all-in-one）
              → （可选）metrics → Prometheus / Grafana
```

**为什么合适（个人/内网 coding agent）：**

| 选项 | 适合度 | 说明 |
|---|---|---|
| **OTel Collector + Grafana Tempo + Grafana** | **首选** | 与厂商无关；Tempo 专吃 traces；Grafana 可同时看日志/指标；内网自托管成熟 |
| **Jaeger all-in-one** | 次选 / 过渡 | 部署极简、UI 够用；长期不如 Tempo+Grafana 可扩展 |
| **SigNoz** | 可考虑 | 一站式 traces+metrics+logs；偏「全家桶」，运维面更大 |
| **凡 traces 都进 Langfuse** | 不推荐作终态 | LLM UI 被基础设施 span 淹没；过滤/告警弱 |
| **Datadog / Honeycomb 云** | 后置 | 个人内网无必要；成本与出站策略另议 |

### 应用侧配置演进（相对 c1475）

今日：`[otel]` 单 endpoint（可直连 Langfuse）。

本 change 意向：

1. **仍推荐默认直连 Langfuse**（你已在用）——仅 agent 语义 span 丰富后（c1480）体验最好。
2. 当需要非 LLM 观测时：把 `[otel].endpoint` 改指向 **Collector**，由 Collector 分流；**或** 配置双 exporter（`traces.llm` / `traces.infra`）——二选一，promote 时定稿。
3. **禁止**为非 LLM 再引入 `tracing` 双栈；继续 fastrace → OTLP。

### Span 命名/属性约定（意向）

| 域 | 示例 span | 后端 |
|---|---|---|
| Agent / LLM | `provider.request`, `react.*`, `tool.execute` | Langfuse |
| Infra | `config.load`, `mcp.connect`, `session.persist`, `bootstrap` | Tempo/Jaeger |
| 共享资源属性 | `service.name=xylitol`, `deployment.environment` | 两边都要 |

分流规则优先用：**span 名 / instrumentation scope / 显式 `xylitol.signal=llm|infra` 属性**（Collector filter），避免靠猜。

## What Changes（仅意向；本 draft 不写代码）

- 产品/架构文档：在 `OTEL与Langfuse观测.md` 增「通用 traces 后端」候补段
- 配置：Collector 示例 `otel-collector.yaml`（或 docs 片段）；可选双 exporter
- 代码：仅在 promote 后——infra 低频 span 补齐 + 分流属性；**不**改 ReAct 主路径合约除非必要

## Out of scope

- c1480 GenAI/Langfuse 属性（独立 change）
- 自研 Inspect UI
- 默认开启远程导出（仍保持显式 opt-in）
- 替用户部署 Tempo/Grafana（文档 + 推荐即可）

## Status

**purpose-draft** — 等 c1475 归档、c1480 推进或你确认后端选型后再 promote（design/tasks/specs）。

## Ethics

- risk_level: low（设计稿）
- prohibited_actions: 默认把全量敏感载荷送任意远端；未经确认选定付费云观测为唯一路径
- required_evidence（promote 时）: 分流规则可测；默认仍 none；文档示例可本地复现
