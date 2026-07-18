---
name: xylitol-inspect-runtime-logs
description: >-
  Token-efficient xylitol observability inspect: provider-trace.jsonl + xylitol.log.
  Use for channel mix-ups, tool-stream lag, react lifecycle spans, or when the user
  asks to check/tail/summarize traces. MUST run scripts/inspect_provider_trace.py
  (or just obs-*) — never Read whole JSONL into context.
---

# xylitol 观测 / Inspect（省 token）

**边界**：只做窄读与分组摘要；schema / 闸门 SSOT → `src/AGENTS.md` Provider、`infra-provider-trace`、c1265 design。
**产品前瞻**：`docs/roadmaps/出口流量检视.md`（检视台）；进程内底座见 `docs/architecture/进程内观测.md`。

## 硬约束（agent）

1. **MUST** 用下方 CLI / `just obs-*`；**MUST NOT** `Read`/`Cat` 整份 `provider-trace.jsonl` 或大段 log。
2. 单次工具输出 **≤ ~40 行**；需要细节再换子命令 + 过滤。
3. 结论只引用：`request_id`、`turn_id`、`kind`、`event`/`variant`、`lag_ms`、lifecycle 名；**勿**贴完整 SSE `text`。
4. 人类要看时间线 UI → 推荐节；**不要**为「好看」在会话里展开 JSONL。

## 默认路径

| 文件 | 用途 |
|------|------|
| `~/.xylitol/logs/provider-trace.jsonl` | raw / mapped / lifecycle（`xylitol.provider_trace.v1`） |
| `~/.xylitol/logs/xylitol.log` | `log` 级别（另用 `tail`/`rg`，勿整文件 Read） |

闸门：debug 默认开；release → `XYLITOL_PROVIDER_TRACE=1`；**永不** stdout/stderr 毁 TUI。

## 分组工具（首选）

全局过滤（放在子命令**前**；`just` 薄封装不传过滤，需过滤时直接调 python）：

| 过滤 | 例 |
|------|-----|
| `--since` | `30m` / `1h` / `90s` / `1h30m` |
| `--request-id` | UUID |
| `--turn-id` | c1265 `react.turn` |
| `--trace-id` | fastrace hex |

```bash
just obs-summary
just obs-requests n=8
just obs-recent n=40
just obs-turns n=8
just obs-lag REQUEST_ID=…              # 默认最近 request
just obs-lifecycle TURN_ID=…           # 或 REQUEST_ID
just obs-channel REQUEST_ID=…

# 带过滤（推荐 agent 直接用这条）
python3 scripts/inspect_provider_trace.py --since 30m summary
python3 scripts/inspect_provider_trace.py --turn-id TID turns -n 8
python3 scripts/inspect_provider_trace.py --request-id RID lag
python3 scripts/inspect_provider_trace.py --path /other/trace.jsonl summary
```

### 推荐排障顺序

| 怀疑 | 命令顺序 |
|------|----------|
| 「有没有 trace / 开没开闸」 | `summary` → `requests` |
| 工具意图晚于 args 流 | `lag`（必要时加 `REQUEST_ID`） |
| c1265 span 有没有 | `turns` → `lifecycle`（需较新二进制：reporter 写入 name/phase/turn_id） |
| thinking/text 通道混 | `channel` + 少量 `recent` |
| 级别日志 | `tail -n 80 ~/.xylitol/logs/xylitol.log`（勿 Read 全文件） |

## 人类：DuckDB 即席查询（可选）

Agent **仍优先** `just obs-*`。人类本地探索可用 [DuckDB](https://duckdb.org/)（勿把大结果贴回 agent 会话）：

```sql
-- 启动: duckdb
SELECT kind, count(*) AS n
FROM read_json_auto(concat(getenv('HOME'), '/.xylitol/logs/provider-trace.jsonl'))
GROUP BY 1 ORDER BY n DESC;

SELECT request_id, count(*) AS n
FROM read_json_auto(concat(getenv('HOME'), '/.xylitol/logs/provider-trace.jsonl'))
WHERE kind IN ('raw','mapped')
GROUP BY 1 ORDER BY n DESC LIMIT 10;

-- 最近 30 分钟 lifecycle（需 name/phase 已落盘）
SELECT name, phase, turn_id, ts_unix_ns
FROM read_json_auto(concat(getenv('HOME'), '/.xylitol/logs/provider-trace.jsonl'))
WHERE kind = 'lifecycle'
  AND ts_unix_ns > (epoch_ns(now()) - 30*60*1e9)
ORDER BY ts_unix_ns DESC
LIMIT 40;

-- 某 request：raw args-delta vs mapped ToolCall*
SELECT kind, event, variant, ts_unix_ns
FROM read_json_auto(concat(getenv('HOME'), '/.xylitol/logs/provider-trace.jsonl'))
WHERE request_id = 'REQUEST_ID_HERE'
  AND (
    (kind = 'raw' AND event LIKE '%function_call_arguments.delta%')
    OR (kind = 'mapped' AND variant IN ('ToolCallStart','ToolCallDelta','ToolCallEnd'))
  )
ORDER BY ts_unix_ns
LIMIT 50;
```

## 人类友好查看（推荐）

| 场景 | 推荐 | 说明 |
|------|------|------|
| **今天（JSONL 本地）** | 本脚本 + `just obs-*` | agent/人共用；最快、零依赖 |
| JSONL 即席 SQL | DuckDB（上节） | 人类探索；agent 仍优先脚本摘要 |
| 行级浏览 | `lnav` / VS Code JSONL | 人类；agent 勿整文件灌上下文 |
| **Phase B+（标准时间线）** | Jaeger all-in-one + `fastrace-opentelemetry`（opt-in） | 浏览器火焰图；需另 change，默认关 |
| LLM/agent 流水线 OTLP JSON | [WideScope](https://widescope.soumendrak.com/editor/) | 拖入 **OTLP/Jaeger JSON**；**不能**直接吃当前 v1 JSONL |
| SaaS | Honeycomb / Grafana Tempo | 团队规模后再说；同样依赖 OTLP 导出 |

**要点**：当前产物是 **专用 JSONL**，不是 OTLP。要「漂亮 UI」要么继续用脚本摘要，要么走 roadmap M6（opt-in OTel → Jaeger）。**禁止**为了 UI 默认开外部导出或打 stdout。

## agent 友好约定

- 先跑 **一个** 子命令，把 stdout **原样**贴给用户/推理，再决定下一步。
- `lag` / `lifecycle` 已够结论时 **停止**，不要再 `recent -n 500`。
- 用户说「打开 Jaeger」→ 说明需 Phase B OTel；短期用 `obs-*`。
