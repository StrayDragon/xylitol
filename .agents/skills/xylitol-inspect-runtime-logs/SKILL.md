---
name: xylitol-inspect-runtime-logs
description: >-
  Token-efficient inspection of xylitol runtime logs and traces
  (~/.xylitol/logs/xylitol.log, provider-trace.jsonl, and other large
  append-only artifacts). Use when diagnosing provider channel mix-ups
  (thinking vs text), debugging with file logs, or the user asks to
  check/tail/summarize traces — never Read whole JSONL/log files into
  context.
---

# xylitol 运行时日志 / trace 省 token 检视

**边界**：只教「怎么窄读」；schema / 闸门 SSOT 见 `src/AGENTS.md` Provider 段与 archive `c1000-add-infra-provider-trace/design.md`。

## 何时用

- 怀疑上游把正文塞进 reasoning / thinking，或适配器映射错分
- 用户说「看一下 log / trace / provider-trace」
- 需要从 `~/.xylitol/logs/` 取证，又要控制上下文体积

## 默认路径

| 文件 | 用途 |
|------|------|
| `~/.xylitol/logs/xylitol.log` | `log` 级别诊断 |
| `~/.xylitol/logs/provider-trace.jsonl` | fastrace 展平后的 raw↔mapped（`schema: xylitol.provider_trace.v1`） |

其它大型 append-only 产物（session JSONL、导出 dump）同样适用下方规则。

## 硬约束（token）

1. **MUST** 用 `rg` / `tail` / 短 `python -c`（或 `just` 封装）；**MUST NOT** 用 Read/Cat 把整文件塞进上下文。
2. 单次工具输出控制在几十行；不够再分页（更大 `tail -n`、或 `rg` + 更窄 pattern）。
3. 对照结论只引用必要字段（如同一 `request_id` 下 `kind=raw` 的 `event` vs `kind=mapped` 的 `variant`），勿贴整段 SSE。
4. 需要 schema 细节时 **指针** 到 design/AGENTS，勿整篇粘贴。

## 推荐命令（先窄后宽）

```bash
# 最近 N 行
tail -n 40 ~/.xylitol/logs/provider-trace.jsonl
tail -n 80 ~/.xylitol/logs/xylitol.log

# provider-trace：kinds / variants 计数
python3 -c "
import json
from collections import Counter
from pathlib import Path
p = Path.home() / '.xylitol/logs/provider-trace.jsonl'
c, v = Counter(), Counter()
for line in p.open():
    if not line.strip():
        continue
    o = json.loads(line)
    c[o.get('kind')] += 1
    if o.get('variant'):
        v[o['variant']] += 1
print('kinds', dict(c))
print('variants', dict(v))
"

# 最近 request_id，再按 id 抽对照
rg -n '"request_id"' ~/.xylitol/logs/provider-trace.jsonl | tail -5
# 通道错分常用 pattern
rg 'response\.(reasoning_text|output_text)\.delta|"variant":"ThinkingDelta"|"variant":"TextDelta"' \
  ~/.xylitol/logs/provider-trace.jsonl | tail -30

# t0718 / c1265：args delta → mapped ToolCall* 滞后
python3 - "$HOME/.xylitol/logs/provider-trace.jsonl" "${REQUEST_ID:-}" <<'PY'
import json, sys
from pathlib import Path
path = Path(sys.argv[1])
rid = sys.argv[2] if len(sys.argv) > 2 else ""
rows = []
for line in path.open():
    if not line.strip():
        continue
    o = json.loads(line)
    if rid and o.get("request_id") != rid:
        continue
    rows.append(o)
if not rid and rows:
    rid = rows[-1].get("request_id") or ""
    rows = [o for o in rows if o.get("request_id") == rid]
raw_t = next(
    (
        o["ts_unix_ns"]
        for o in rows
        if o.get("kind") == "raw"
        and "function_call_arguments.delta" in (o.get("event") or "")
    ),
    None,
)
map_t = next(
    (
        o["ts_unix_ns"]
        for o in rows
        if o.get("kind") == "mapped"
        and o.get("variant") in ("ToolCallStart", "ToolCallDelta")
    ),
    None,
)
print("request_id", rid)
print("t_first_args_delta_ns", raw_t)
print("t_first_mapped_tool_ns", map_t)
if raw_t is not None and map_t is not None:
    print("lag_ms", (map_t - raw_t) / 1e6)
elif raw_t is None and map_t is not None:
    print("note", "mapped tools without args-delta raw")
elif raw_t is None and map_t is None:
    textish = any(o.get("variant") == "TextDelta" for o in rows)
    print("note", "no native tool stream; text-only" if textish else "no tool signals")
print("lifecycle_events", sum(1 for o in rows if o.get("kind") == "lifecycle"))
PY
```

## 闸门提醒

- debug 构建：文件日志 / provider trace 默认开
- release：`XYLITOL_DEBUG` / `RUST_LOG`（级别日志）；`XYLITOL_PROVIDER_TRACE=1`（timeline）
- **永不**依赖 stdout/stderr 日志（毁 TUI）
- c1265：开闸时可见 `react.turn` / `react.stream` / `tool.execute` 的 lifecycle Event（与 raw/mapped 同文件）
