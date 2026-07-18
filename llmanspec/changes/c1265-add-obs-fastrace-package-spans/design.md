# Design: c1265 obs fastrace package spans

## Span 树（建议）

```text
provider.request                    (existing)
  └── adapter.map_event             (per SSE → assistant event)
react.turn
  ├── react.stream
  └── tool.execute { name, id }
tui.host.event { kind }             (debug / sampled)
  └── tui.bridge.apply
```

关联：沿用 `request_id` / fastrace `trace_id`；TUI span 用 message/turn id 属性。

## 真机剧本（opt-in）

```text
1. 启用 XYLITOL_PROVIDER_TRACE=1
2. 用户：简单问候
3. 用户：触发 ls/read 类工具
4. 检查 provider-trace：
   - 若存在 function_call_arguments.delta（或 Completions tool_calls）
     → 首个对应 mapped ToolCallStart/Delta 不得晚于「整包 item.done only」模式
   - 若仅有 TextDelta 且含伪 XML → 报告「端点未发原生 tools」，非 UI bug
```

## 与 inspect skill

更新 `xylitol-inspect-runtime-logs`：增加「按 request_id 计算 args_delta→mapped 滞后」的短 python 片段（t0718 分析方法固化）。
