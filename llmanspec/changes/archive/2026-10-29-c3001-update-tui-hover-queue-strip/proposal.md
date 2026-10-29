---
depends_on: []
title: TUI 悬停语义区块：队列条 hover 与点击还原
needs_specs_change: true
branch: sdd/c3001-update-tui-hover-queue-strip
base_branch: main
base_sha: cdcfcc8bc38f87faa5aa9a7122a5f6a52c9f9c75
---

# TUI 悬停语义区块：队列条 hover 与点击还原

## Why

- #24 的 M4 要求鼠标悬停到工具块、折叠簇、队列条等语义区块时给出可点暗示，点击就地展开或聚焦。
- 现行 hover 命中表已覆盖 Tool / Diff / Ask / Thinking / OutputViewport / Compaction / Cluster / LiveTail（`fold_hit.rs`），但 **队列条没有命中区**：鼠标划过 Steering / Follow-up 行没有任何暗示，点击也不会聚焦或还原。
- 键盘 Alt+Up 已能把全部排队文本还原进 editor；鼠标路径应复用同一语义，避免两套行为。

## What Changes

- `UiRoot` 在渲染时记录队列条屏幕起点与行数，并在 `overlay_hits` 增加 `QUEUE_STRIP_HIT_ID`（Chrome 命中区，覆盖队列条整宽）。
- 队列条命中区同时登记 hover 区，复用既有 `apply_hover_tint` 弱强调（不引入洗底 / 反色）。
- 左键点击队列条：
  - 将全部 steer / follow-up 文本还原进 editor（与 Alt+Up 同语义，含当前 editor 草稿拼接）。
  - 清空队列并聚焦 editor。
- 抽取 `HostSession::dequeue_to_editor()`，键盘 Alt+Up 与鼠标点击共用，消除双路径。
- 规格：
  - `app-tui-input` 新增 `queue-strip-hover-click` 规则与场景。
- 测试：
  - 单测：队列条命中区覆盖整宽且 hover/click 命中同一 id。
  - BDD：队列条点击还原文本并清队。

## Capabilities

- `app-tui-input`（队列条 hover / 点击还原）

## Impact

- **用户可感知**：队列条可点暗示与键盘路径一致；鼠标用户可点击还原排队消息。
- **不变**：键盘 Alt+Up 仍是主路径；工具块 / 折叠簇 hover 行为不变；activity-fold 多行块不新增逐块 hover。
- **可回退**：移除队列条命中区即可回到纯键盘路径。

## Non-Goals

- 不引入全局背景洗底或反色 hover。
- 不把 hover 套到 activity-fold 多行折叠块的每个工具叶上。
- 不改变队列条文案、Alt+Up 键位或队列 FIFO 语义。
- 不为 gpui 等未开放端预埋第二套词表。
