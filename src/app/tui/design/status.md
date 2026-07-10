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

## MUST

1. idle：**MUST NOT** 占行（不要空转 spinner）。
2. busy：至多一行 `spinner + 短词`（Working / Running tool / Retry…）。
3. **MUST NOT** 放 turn 计数、耗时百分比、双列元数据。

颜色：`{colors.muted}`；忙碌 spinner 可用 `{colors.accent}`（一屏最多一处 accent）。
