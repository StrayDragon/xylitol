# Design — c1000（fastrace 单栈迁移 + provider trace）

## 决策（相对上一版）

| 旧草案 | 新决策 |
|--------|--------|
| 在现有 `tracing` 上加 provider Layer | **废弃 tracing 栈** |
| fastrace 留 future | **本 change 落地 fastrace，并迁移** |
| tracing + fastrace 并存 | **禁止双栈** |

## 为什么 fastrace 适合做基础设施

- **库级 / 关闭零开销**：依赖不带 `enable` 时可被编译掉；应用带 `enable`，未 `set_reporter` 时记录可忽略。
- **Event = 挂在 span 上的点**：天然表达 raw SSE 与 mapped chunk，同一 `SpanContext` / root 对齐。
- **高性能路径**：单线程流解析用 `LocalSpan`；跨 await 用 `Span` + `FutureExt::in_span`。
- **官方不做第二套 log 宏**：级别日志走 **`log`**，并可挂到当前 span（`logforth` / `fastrace` 文档路径）。本仓现有 `tracing::warn!/info!/debug!` ≈47 处 → **`log::`**。

## 与「级别日志」的分工

```text
fastrace  ——  因果时间线（request span、provider raw/mapped Event）
log       ——  级别诊断（warn/info/debug），file-only，可附到当前 span
```

这是 fastrace 推荐架构，**不是**双 tracing 栈。

## 禁止

- `ConsoleReporter`（默认 stderr）→ **毁 TUI**；必须自研 **FileReporter**（同步 append，`0o600`）。
- 残留 `tracing` / `tracing-subscriber`。

## 闸门

| | debug | release |
|--|-------|---------|
| fastrace Reporter | 默认装（provider + 通用 span 可写 `trace.jsonl` / `provider-trace.jsonl`） | 默认不装；`XYLITOL_PROVIDER_TRACE=1` 装 |
| `log` 文件 | 对齐 ath3：默认开（或随 Reporter） | `XYLITOL_DEBUG` / `RUST_LOG` 等价（用 `env_logger`/`logforth` 的 filter） |

实现细节：单 crate 可始终 `fastrace` feature=`enable`，**运行时**不 `set_reporter` = 关闭；避免为 debug/release 拆两套 Cargo feature 矩阵。

## Provider 对照

```text
Span::root("provider.request", …)  properties: request_id, api, model
  Event { name: "raw", … event_type, delta… }
  Event { name: "mapped", … variant: ThinkingDelta|TextDelta, text… }
→ FileReporter → ~/.xylitol/logs/provider-trace.jsonl
```

Agent 读同一 `request_id` 下 raw vs mapped 即可判责。

## JSONL schema（v1，留扩展）

**文件**

| 文件 | 内容 |
|------|------|
| `logs/xylitol.log` | `log` 级别日志（人类/agent tail） |
| `logs/provider-trace.jsonl` | 一行一 JSON 对象；UTF-8；同步 append |

**每行公共字段（v1）**

| 字段 | 类型 | 说明 |
|------|------|------|
| `schema` | string | 固定 `"xylitol.provider_trace.v1"` |
| `ts_unix_ns` | u64 | 事件时间 |
| `trace_id` | string | fastrace TraceId（hex） |
| `span_id` | string | fastrace SpanId（hex） |
| `request_id` | string | 业务相关 id（uuid）；对照主键 |
| `kind` | string | `raw` \| `mapped` \| `span` |
| `api` | string | 方言：`openai-responses` / `anthropic-messages` / `openai-completions` |
| `model` | string | 模型 id |
| `ext` | object | **预留**；v1 可为空对象，日后加字段不改 schema 主版本时优先塞这里 |

**kind 特有**

| kind | 字段 | 说明 |
|------|------|------|
| `raw` | `event` | 协议事件名（如 `response.reasoning_text.delta`） |
| `raw` | `text` | delta/载荷片段（截断后） |
| `mapped` | `variant` | `ThinkingDelta` / `TextDelta` / `FunctionCall` / `Done` / … |
| `mapped` | `text` | 映射后文本（截断后） |
| `*` | `truncated` | bool；`text` 被截断时为 true |

**截断**：`text` 默认上限 **4096** Unicode 标量；超出截断并 `truncated:true`。常量名 `PROVIDER_TRACE_TEXT_MAX`，日后可经 env 覆盖（本 change 可硬编码 + 测）。

**扩展规则**：新增可选字段放 `ext` 或 bump `schema` → `v2`；读者应忽略未知字段。

## 迁移步骤（tasks 顺序）

1. 引入 fastrace + log + FileReporter；组合根装配；TUI 冒烟无 stderr 污染
2. 批量 `tracing::` → `log::`；删 tracing 依赖；改 `infra-logging` 合约
3. provider emit + 三适配器 + 单测

## Future

- OTel reporter 可选 feature
- Tail sampling（`Span::cancel`）只留异常 turn
- `max_level` 类编译期裁剪若 release 仍嫌重
