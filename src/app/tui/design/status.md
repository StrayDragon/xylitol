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

1. idle：**MUST NOT** 显示 spinner / “Ready” 忙碌文案。允许 **1 行空白**作为 editor 上方呼吸间距（对齐 pi `IdleStatus` / `Loader` 前导空行）；**不要**把空白算成 status chrome。
2. busy：`Loader` 形态 = **前导空行 +** 一行 `spinner + 短词`（Working / Running tool / Retry…），紧贴 input。
   - Spinner **MUST** 按 `Loader::interval_ms`（默认 ~80ms）推进；host ~16ms idle_tick **MUST NOT** 每 tick 都 `Loader::tick`（否则会异常快）。
3. **MUST NOT** 放 turn 计数、耗时百分比、双列元数据。
4. 队列徽章（steer/follow-up）：**不进** status 行；进 footer 前缀 `q:sN|fM`，全文进 **scrollback 与 status 之间** 的 dim 队列块（见 [`queue-steer.md`](./queue-steer.md)）。**MUST NOT** 写成 scrollback `[steer]` 系统墙。

颜色：`{colors.muted}`；忙碌 spinner 可用 `{colors.accent}`（一屏最多一处 accent）。
