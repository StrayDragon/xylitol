---
version: "alpha"
name: "status"
description: "Idle-hidden busy status row with optional accent spinner."
tokens_from: "../DESIGN.md"
components:
  status-line:
    textColor: "{colors.muted}"
    height: "{spacing.status-rows}"
  status-spinner:
    textColor: "{colors.accent}"
---

# Status

> Token 根源：`{colors.*}` / `{spacing.*}` → [`../DESIGN.md`](../DESIGN.md)。
> **c475**：产品 host 必须落地本文件；playground 槽 Full shell / Layout 可预览。

## MUST

1. idle：**MUST NOT** 占行（不要空转 spinner；高度 0）。
2. busy：至多一行 `spinner + 短词`（Working / Running tool / Retry…）。
3. **MUST NOT** 放 turn 计数、耗时百分比、双列元数据。
4. 队列徽章（steer/follow-up）：**不进** status 行；进 footer 前缀或 editor 上方一行 hint（见 [`queue-steer.md`](./queue-steer.md)）。

颜色：`{colors.muted}`；忙碌 spinner 可用 `{colors.accent}`（一屏最多一处 accent）。
