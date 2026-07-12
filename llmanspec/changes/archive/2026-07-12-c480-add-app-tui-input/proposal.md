---
change_id: c480-add-app-tui-input
title: "app-tui-input：Editor、slash、steer/follow-up 键位"
status: ready
priority: 480
depends_on:
  - "c460-add-app-tui-host"
  - "c461-expose-steer-followup-seam"
  - "c455-add-package-tui-input-listener"
  - "c465-add-app-tui-bridge"
author: agent
track: B
---

# c480-add-app-tui-input

## Why

chrome / scrollback / trust 已通，但产品 Host 仍缺 DESIGN `keybindings.md` MVP：忙碌 Enter→steer、Alt+Enter→follow-up、Esc→abort、idle `/exit` `/model`。demo 已验形态；本变更把同一键位接到 `Driver` + `dispatch`。

## What Changes

1. **Host 键位**：忙碌 Enter → `Driver::steer`；Alt+Enter → `Driver::follow_up`；Alt+Up → 还原队列到 editor + `clear_queue(true,true)`；Esc（流中）→ `abort` + `clear_queue(steer=true, follow_up=false)`；已有 Ctrl+C / 双 Esc stub 保持。
2. **Idle Enter**：非 `/` → `Driver::run`（已有）；`/exit` → 退出 TUI；`/model` → `dispatch(SetModel|CycleModel|GetAvailableModels)` 极简；未知 `/` → scrollback 系统错误，不崩。
3. **队列 chrome（硬约束）**：非空时在 scrollback 与 status 之间画 dim `Steering:` / `Follow-up:` + `↳ Alt+Up…`（对齐 pi）；footer `q:sN|fM`；**禁止** scrollback `[steer]` 系统墙（见 `design/queue-steer.md`）。
4. **单测**：host harness 覆盖 steer/follow-up/Alt+Up/abort/slash；不扩 c491 stub。

## Capabilities

- `app-tui-input`：产品 host 键位接线（ati2/ati3 + 产品场景）
- `app-tui-commands`：`/exit` `/model` 经 dispatch（atm1/atm2）

## Out of scope

- palette/settings 全量、bash `!`（c492）、真 session 树、CompletionSource 弹层（可后置；MVP 可无 popup 仅提交解析）

## Impact

- 触达：`src/app/tui/{host,mod,ui_root,tests}.rs`；只经 `Driver` / `dispatch`。
- 风险：Esc 与双 Esc 时间窗冲突 → 忙碌时 Esc 优先 abort，不走树。
