---
version: "alpha"
name: "queue-steer"
description: "Steer / follow-up queue strip — 对齐 pi pendingMessagesContainer；产品接线 c480/c461。"
tokens_from: "../DESIGN.md"
components:
  queue-hint:
    textColor: "{colors.muted}"
---

# Queue / steer

> Token 根源：`{colors.*}` → [`../DESIGN.md`](../DESIGN.md)。
> 形态参考：pi `interactive-mode.ts` → `updatePendingMessagesDisplay`（`Steering:` / `Follow-up:` + dequeue hint）。

## MUST（产品 · c480）

1. **忙碌 Enter = steer**：入队且 **MUST NOT** 打断当前轮；刷新 footer `q:sN|fM`。
2. **Alt+Enter = follow-up**：入队；**MUST** 仅在完全空闲后开新轮。
3. **Strip 位置（硬约束）**：队列非空时，在 **scrollback 与 status 之间** 画 dim 块（与 pi 同形）：
   - 每条 steer → `Steering: {text}`
   - 每条 follow-up → `Follow-up: {text}`
   - 末行 hint → `↳ Alt+Up to edit all queued messages`
4. **上行（硬约束）**：消息从队列 **注入 agent history** 时，MUST 经 `XyEvent::MessageStart/End { role: user }` 进入 scrollback 为普通 `UiEntry::User`（与 idle 提交同形）；queue strip 随 `QueueUpdate` 消退后，用户内容仍 MUST 留在 transcript。**MUST NOT** 只在 strip 里闪一下就消失。
5. **MUST NOT** 把排队内容写成 scrollback `System` / `[steer]` / `[follow-up]` 墙（那是 demo 假树可观测痕迹，**不是**产品 queue strip）。
6. **footer**：队列非空时前缀 `q:sN|fM`（dim）；**MUST NOT** 把全文塞进 status 行（见 [`status.md`](./status.md)）。
7. **Alt+Up**：把两侧队列文本还原进 editor（与当前草稿用空行拼接），并 `clear_queue(steer+follow_up)`。
8. 键位见 [`keybindings.md`](./keybindings.md)。

## 产品接线

逻辑 seam：c461（`Driver::steer` / `follow_up` / `clear_queue`）；输入面：c480。demo 假队列仅形态学实验，**不得**当作产品 queue strip SSOT。
