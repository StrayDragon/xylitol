---
version: "alpha"
name: "status"
description: "Idle-hidden busy status row with optional accent spinner and next-turn cue."
tokens_from: "../DESIGN.md"
components:
  status-line:
    textColor: "{colors.muted}"
    height: "{spacing.status-rows}"
  status-spinner:
    textColor: "{colors.accent}"
  status-next-turn-cue:
    textColor: "{colors.muted}"
---

# Status

> Token 根源：`{colors.*}` / `{spacing.*}` → [`../DESIGN.md`](../DESIGN.md)。
> **c475**：产品 host 必须落地本文件；playground 槽 Full shell / Layout 可预览。
> **下轮预告**（next-turn cue）：[`pending-runtime.md`](./pending-runtime.md)；词表 [`docs/architecture/TUI信息面与chrome词汇.md`](../../../../docs/architecture/TUI信息面与chrome词汇.md)。
> **壳层通告**（chrome toast）：status **上方**独立行，见 [`chrome-toast.md`](./chrome-toast.md)（≠ 本文件 status 行 / ≠ ScrollNotice）。

## MUST

1. idle：**MUST** 显示恰好 **1 行空白**作为 editor 上方呼吸间距（对齐 pi `IdleStatus` / `Loader` 前导空行 / agent_demo）；**MUST NOT** 显示 spinner / “Ready” 忙碌文案；**不要**把空白算成 status chrome 文案。
2. busy：`Loader` 形态 = **前导空行 +** 一行 `spinner + 短词`（Working / Assembling / Running tool / Retry…），紧贴 input；**MUST NOT** strip 前导空行。
   - Spinner **MUST** 按 `Loader::interval_ms`（默认 ~80ms）推进；host ~16ms idle_tick **MUST NOT** 每 tick 都 `Loader::tick`（否则会异常快）。
3. **MUST NOT** 放 turn 计数、耗时百分比、双列元数据。
4. **队列条**（steer/follow-up）：**不进** status 行；进 footer 前缀 `q:sN|fM`，全文进 **scrollback 与 status 之间** 的 dim 队列块（见 [`queue-steer.md`](./queue-steer.md)）。**MUST NOT** 写成 scrollback `[steer]` 滚动提示墙。

## 下轮预告（可复用槽）

> 把 busy 行右侧空旷收成 **边界前常驻** 下轮预告，而不是另起 strip 行。首用例 = 换模 / thinking NextTurn（[`pending-runtime.md`](./pending-runtime.md)）。

5. **形态**：busy 内容行 = **左组** `spinner + 短词`（`.status-lead`，**MUST** 紧贴左缘、同组不可拆到行尾）+（可选）**右组** dim 下轮预告（`.status-next-turn-cue`，`margin-left: auto` / 右对齐）。**MUST NOT** 对 lead 内 spinner 与短词做 `space-between`（会把 Working 顶到行尾——回归）。窄宽 **优先截断下轮预告**，保留左侧短词。**MUST NOT** 为下轮预告增加第二行 status。
6. **寿命**：下轮预告表示「相对当前 in-flight / 本 turn 生效中，选中尚未在下一边界生效」的差；常驻到消费或收敛（见 pending-runtime），**不是** ephemeral toast。
7. **与左侧短词解耦**：`Assembling` → `Working` → `Running tool` → `Compacting` → `Retry…` / `Follow-up pending` 轮换时，下轮预告槽 MUST 保留（除非 pending 已清除）。**MUST NOT** 因 `set_busy_status` 整行重写而丢掉下轮预告。
8. **适用 busy 种类（钉死）**：
   - **Agent run busy**（`run_active` / in-flight LLM）：允许模型 / thinking 下轮预告。
   - **仅 bang busy**（`bash_active` 且无 agent run）：**MUST NOT** 显示 `Next turn:` 类下轮预告（无 LLM NextTurn 语义）；换模 / thinking 按 **无 in-flight** 处理（footer 直接跟选中）。
9. **槽内只放「选中 vs 生效中」差**：M0 = 模型 / thinking。**MUST NOT** 把 debug、临时提示、steer 原文塞进下轮预告。能力覆盖等复用本槽前须另开 design 条款。
10. **idle**：无 status 内容行 → **无下轮预告**；进入 idle / abort 落定后若无 in-flight，pending MUST 已收敛（见 pending-runtime），禁止「选中已变、无 chrome、状态机仍 pending」幽灵。

颜色：`{colors.muted}`；忙碌 spinner 可用 `{colors.accent}`（一屏最多一处 accent）；下轮预告用 muted，**MUST NOT** 再占第二处 accent。
