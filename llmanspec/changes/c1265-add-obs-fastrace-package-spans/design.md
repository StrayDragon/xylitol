# Design: c1265 Phase A（收窄）

## 价值分层

| 层 | 作用 | Phase |
|---|---|---|
| 已有 raw/mapped Event | t0718 滞后判定的数据面 | 已有 |
| inspect skill 脚本 | 把 t0718 变成可复制命令 | **A** |
| `react.stream` / `react.turn` / 可选 `tool.execute` | 跨层时间边界（低频） | **A** |
| `adapter.map_event` / TUI / engine / live CI | 热路径或产品断言 | **B+** |

## Span 树（Phase A）

```text
react.turn                          (optional parent)
  └── react.stream                  (1 per provider HTTP)
        └── provider.request        (existing; may stay Span::root —
                                    correlate via request_id property)
              ├── Event raw
              └── Event mapped
tool.execute {name,id}              (sibling / under turn; optional)
```

跨 crate 真父子若成本高：**属性关联 `request_id` 优先**，不阻塞 Phase A。

## Reporter 注意

`FileTraceReporter` 今日只展平 **span.events**。无 Event 的 agent span 可能不进 `provider-trace.jsonl`。

选项（实现时二选一，最小改动）：

1. 在 `react.stream` 起止各 `add_event`（`kind=lifecycle`）
2. 扩展 reporter 写 span start/end 一行（schema 小增，需改 ipt1）

优先 (1)，避免动 schema。

## 关闸

复用 bridge `provider_trace_active()`（或 agent 同原子/同一 env）：关则 `start()` 返回 None，不做序列化。

## Inspect 配方（skill）

输入：`~/.xylitol/logs/provider-trace.jsonl` + `request_id`。

输出示例：

- `t_first_args_delta`（raw `response.function_call_arguments.delta` 或 Completions 等价）
- `t_first_mapped_tool`（mapped `ToolCallStart` / `ToolCallDelta`）
- `lag_ms`；若无 raw tool delta 仅有伪 XML TextDelta → 报告「端点未发原生 tools」

## 非目标

- 每 SSE 子 span
- TUI 采样框架（留给 Phase B）
- BDD 可执行场景接线（feature 已有；steps 另开，不挡 Phase A 文档+单测）
