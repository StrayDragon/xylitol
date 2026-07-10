# Status

## MUST

1. idle：**MUST NOT** 占行（不要空转 spinner）。
2. busy：至多一行 `spinner + 短词`（Working / Running tool / Retry…）。
3. **MUST NOT** 放 turn 计数、耗时百分比、双列元数据。

颜色：`muted`；忙碌 spinner 可用 `accent`（一屏最多一处 accent）。
