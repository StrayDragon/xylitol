---
depends_on: []
---

# 通用 Tab 与子 agent 进入视图

## Why

包里没有 Tab / Pager。产品 Down 现在是 editor 光标或 prompt-history，槽里是 `tui.select.down`。opencode 把顶栏 tab（`session.tab.*`）和 Down 开子 agent picker（`session.child.first`）分成两套键。pi 和 crush 都没有 in-app tab。ratatui `Tabs` 只画 title 行，不处理键和鼠标。

`docs/roadmaps/Sub-Agent编排.md` 还是提案。`src/AGENTS.md`：未来子 agent = 另建隔离 runtime。fork 树不是活的子 worker。没有 runtime 就不能做产品 Down 进子面。

## What Changes

本票只做刀 A。刀 B 整段 park。

### 刀 A — 包 `Tabs`

- 新组件 `xylitol-tui::Tabs`：N 个 title、一个 `selected` 下标。可以挂与 title 对齐的 page 子组件；只渲染当前 page。
- 不知道 session id，不读 Host / `XyEvent`。
- 键：`tui.tabs.prev` / `tui.tabs.next`（默认 left / right，循环）；无修饰的 `1`–`9` 选 1-based 下标（超出 title 数则忽略）。只在组件收到 `handle_input` 时生效（焦点由 host / TUI 给）。
- `render(width)` 之后 `title_col_ranges()` 给出每个 title 在条上的列区间，给以后 hit map 用。本票不接鼠标、不登记 `HitRegion`。
- `Container` 仍是垂直栈。`Tabs` 是普通 `Component`，可以放进 `Container`。
- 输出 `Vec<String>` ANSI。主题用闭包。不引入 `StyledLine`，不依赖 ratatui `Tabs`。

### 刀 B — park

产品 Down 开 picker、子面 Up/左右切兄弟、多根会话顶栏、点 tab / picker 行：等编排进 architecture，另开 change。可点路径那时再 `depends_on` 已归档的 c2858。

## 非目标

- 不改产品 host Down / editor 历史 / `/model` 左右切档。
- 不在 Host 预埋第二套动作 id。
- 不把 fork 树写成活子 agent。
- 不常驻顶栏快捷键墙。
- 不做图内 pan、不做双栏。
- 不把 ratatui-markdown hybrid scroll 当成 Down-enter。

## Capabilities

- `package-tui-tabs`（新）
- 不改 `app-tui-host` / 不新建 `app-tui-subagent`

## Impact

- 默认 left/right 只在 `Tabs` 有焦点时消费。`tui.select.pageUp/pageDown` 默认仍未绑 left/right（c1470 留给 `/model`）。
- 产品接线、`agent_demo` 当 SSOT：都不在本票。`agent_demo` 以后可以挂一块试跑，不是 designing。

## Open Questions

1. ~~首切片是否只做包 Tab？~~ **只做刀 A**（2026-10-11）。刀 B 无 runtime。
2. ~~Down 门控：有孩子且 editor 空 vs 另开和弦？~~ **推迟到刀 B**。
3. ~~产品顶栏多会话 tab？~~ **否**。默认仍一次一会话 + resume 槽。

## Further Notes

- [research/tabs-multi-agent-peers.md](./research/tabs-multi-agent-peers.md)
- `docs/roadmaps/Sub-Agent编排.md` · delayed c2325
