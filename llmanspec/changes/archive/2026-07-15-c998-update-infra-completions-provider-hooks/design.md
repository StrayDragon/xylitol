# Design — c998

## 策略 A

对齐 `openai_responses::send_request`：自建 JSON body → `run_before_headers` / `run_before_request` → reqwest → `run_after_response` → 解析流。

可保留 async-openai 仅作类型/映射参考，但热路径 HTTP 必须可控。

## 验收

- 注入 dispatcher：Modify 改 body 字段可见
- 空 hooks：无脚本进程
- 既有 Completions 单测/集成不红
