---
version: "alpha"
name: "queue-steer"
description: "Steer / follow-up queue chrome — demo 已验证（c468）；产品接线见 c480/c461。"
tokens_from: "../DESIGN.md"
components:
  queue-hint:
    textColor: "{colors.muted}"
---

# Queue / steer

> Token 根源：`{colors.*}` → [`../DESIGN.md`](../DESIGN.md)。

## MUST（demo 已验证 · c468）

1. **忙碌 Enter = steer**：入队且 **MUST NOT** 打断当前轮脚本/流式调度；可写 transcript / 活树 `[steer]` 标记。
2. **Alt+Enter = follow-up**：入队；**MUST** 仅在完全空闲后开新轮（不提前挂 user 树节点）。
3. **footer**：队列非空时展示 `steer:N` / `follow-up:N`（dim）。
4. 键位见 [`keybindings.md`](./keybindings.md)。

## 产品接线

逻辑 seam：c461（`Driver::steer` / `follow_up`）；输入面：c480。demo 假队列仅形态学，不替代 Driver。
