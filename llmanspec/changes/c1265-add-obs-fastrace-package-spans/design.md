# Design: c1265 Phase A + 前瞻钩子

## 价值分层

| 层 | 作用 | Phase |
|---|---|---|
| 已有 raw/mapped Event + JSONL | t0718 数据面 | 已有 |
| inspect skill 滞后脚本 | 可复制排障 | **A** |
| `fastrace-futures` + `react.stream` / `react.turn` | 跨 await local parent + 低频边界 | **A** |
| 可选 `tool.execute` | 工具执行边界 | **A 可选** |
| opt-in OTel / Jaeger reporter | 标准导出，与 JSONL 并存 | **B**（钩子，不实现） |
| TUI/engine / map_event / live CI | 热路径或产品断言 | **B+** |

## fastrace 生态取舍（SSOT）

| 采用 | 理由 |
|---|---|
| `fastrace` + `#[trace]` / `enter_with_local_parent` | 关闸零开销埋点 |
| `fastrace-futures`（`StreamExt::in_span`） | 包 provider `Stream`；poll 时自动 `set_local_parent` |

| 不用（A） | 理由 |
|---|---|
| `fastrace-opentelemetry` / jaeger / datadog | 导出向；与专用 JSONL 并行另开；默认禁 |
| `fastrace-reqwest` | 模型厂商不接我们的树；无助于本地 lag |
| `fastrace-axum` / poem / tonic / tower | 非本波主路径 |
| `fastrace-tracing` | 违反 ipt3 |

**保留自研**：`FileTraceReporter` + `xylitol.provider_trace.v1` + `emit_raw`/`emit_mapped`——领域对照格式，生态 reporter 替代不了。

## Span 树（Phase A）

```text
react.turn                 (Span::root + turn_id；不跨 await 持 LocalParentGuard)
react.stream               ← fastrace-futures::in_span（仅 poll 内 local parent）
provider.request           (existing root；用 request_id / 时间邻近关联)
  ├── Event raw
  └── Event mapped
tool.execute {name,id}     (Span::root；可选)
```

`async_stream` ReAct 循环是 `Send` 的：`LocalParentGuard`（`Rc`）**禁止**跨 `.await` 持有。关联靠 `turn_id` 属性 + lifecycle Event。

## Reporter

今日只写 **events**。Phase A：优先在 `react.stream` 起止 `add_event(kind=lifecycle)`。

Phase B 钩子：`CompositeReporter` 或 env 切换——`FileTraceReporter` **永远可关可开**；OTel 仅 `XYLITOL_OTEL_*=1` 类显式开启，且不得写 stdout/stderr。

## 与「出口流量检视」roadmap

```text
Phase A（本 change）     进程内时间线 + 本地对照文件 + skill
        ↓ 准备
Inspect M1 事实源         会话可关联的第一方出站（产品面）
        ↓ 可选
Phase B OTel              同一时间线导出给 Web Inspect / 外部后端
```

产品文案只在 `docs/roadmaps/出口流量检视.md`；实现合约仍在本 change / ipt*。

## Inspect 配方（skill）

- 输入：`provider-trace.jsonl` + `request_id`
- 输出：`t_first_args_delta`、`t_first_mapped_tool`、`lag_ms`；无原生 tools 时报告端点形态

## 非目标（A）

- 每 SSE 子 span；TUI 采样；默认 OTel；BDD steps 接线（不挡文档+单测）
