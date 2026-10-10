---
depends_on: []
---

# 通用 Tab 与子 agent 进入视图

## Why

xylitol 无 Tab/Pager，也无活的多 agent 面。Down 是 editor 光标/历史或 `tui.select.down`。opencode 把 **顶栏 tab**（`session.tab.*`）和 **Down = 子 agent picker**（`session.child.first`）分成两套键。pi 两者都无。

ratatui `Tabs` 是 **纯 paint**：`select(usize)` + 水平 title，**不处理键或鼠标**；demo 用 Left/Right。要点击得 app 自己按 title 宽度切 `mouse.column`。xylitol 刀 A 应对齐这个分工：包只画 + 键，可点 tab 挂 c2858 的 dock/chrome region，不要做 StatefulWidget。

Sub-Agent 编排仍是 roadmap；runtime = 另建隔离 runtime。fork ≠ 活的子 worker。

## What Changes

### 刀 A — 包：哑 Tab/Pager（不依赖 c2858）

- N 标签、一次一页、Left/Right 或 1–9；不知 session id。
- 可暴露「第 i 个 title 的列区间」供日后 hit map 用，但本刀不接鼠标。
- `Container` 仍是垂直栈。

### 刀 B — 产品：子 agent 进入（编排 runtime 未就绪则 park）

- 有孩子且 editor 空闲/非槽时，Down 开 picker；否则保留光标/历史。
- 子面内 Up = 父；左右切兄弟。Host 拥有 running/done/merge；picker 是 client 动作（c2325）。
- 多根会话顶栏另议；默认仍一次一会话 + resume 槽。
- **可点 tab / picker 行**：`depends_on` c2858，本刀 B 键盘路径可不依赖。

## 非目标

- 不在 Host 预埋第二套动作 id（gpui 未开闸）。
- 不把 fork 树伪装成活子 agent。
- 不常驻顶栏快捷键墙。
- 不把 ratatui-markdown hybrid scroll（free/engaged）当 Down-enter。

## Capabilities

- `package-tui-tabs`（刀 A）
- `app-tui-host` / 新 `app-tui-subagent`（刀 B；编排 MUST 进 architecture 后再落地）

## Impact

- Down 与 `tui.editor.cursorDown` / 历史冲突，必须门控。
- 刀 A 可与 c2858 / c2860 并行。

## Open Questions

1. 首切片是否只做包 Tab（推荐）？
2. Down 门控：有孩子且 editor 空 vs 另开和弦？
3. 要不要产品顶栏多会话 tab？

## Further Notes

- [research/tabs-multi-agent-peers.md](./research/tabs-multi-agent-peers.md)
- `docs/roadmaps/Sub-Agent编排.md` · delayed c2325
