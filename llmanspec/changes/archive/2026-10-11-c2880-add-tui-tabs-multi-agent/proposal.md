---
depends_on: []
branch: sdd/c2880-add-tui-tabs-multi-agent
base_branch: main
base_sha: b2fc101a328d47e702e76cf3f5b879574b637ce4
needs_specs_change: false
---

# 通用 Tab 与子 agent 进入视图

## Why

包里没有 Tab / Pager。产品 Down 现在是 editor 光标或 prompt-history，槽里是 `tui.select.down`。opencode 把顶栏 tab（`session.tab.*`）和 Down 开子 agent picker（`session.child.first`）分成两套键。pi 和 crush 都没有 in-app tab。ratatui `Tabs` 只画 title 行，不处理键和鼠标。

`docs/roadmaps/Sub-Agent编排.md` 还是提案。`src/AGENTS.md`：未来子 agent = 另建隔离 runtime。fork 树不是活的子 worker。没有 runtime 就不能做产品 Down 进子面。

## What Changes

本票只做刀 A。刀 B 整段 park。

### 刀 A — 包 `Tabs`

- 新组件 `xylitol-tui::Tabs`：只画标题条。状态是 N 个标题 + 一个 `selected` 下标。不持有对话条目或其它页；产品端按 `selected` 自己换内容。
- 不知道 session id，不读 Host / `XyEvent`。
- 溢出：窗口 + 两侧计数（Q5）。切到上一标题 / 下一标题：登记 `tui.tabs.prev` / `tui.tabs.next`，默认不绑任何物理键（Q6）。产品端以后要切会话书签时，再在端侧绑定表里写上键。
- `render(width)` 之后 `title_col_ranges()` / `overflow_col_ranges()` 给出可见标题与溢出标记的列区间。左键与条上滚轮改 `selected`（`offer_pointer`，焦点前投递）。不登记 `HitRegion`。
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
- `package-tui-engine`（r1614 收窄 + r1946 指针投递）
- 不改 `app-tui-host` / 不新建 `app-tui-subagent`

## Impact

- 用户按 Left、Right、1–9 时，行为与今天相同：editor 移光标；`/model` 槽里切档。标题条默认不收这些键。
- 产品端本票不接线。以后用 `selected` 下标对照会话书签（当前对话条目），再对照子 agent 端。标题条不持有这些内容。
- 左右分列、整条隐藏：以后的想法，本票 API 不预留字段。
- 可执行合约 SSOT：`packages/xylitol-tui/llmanspec/specs/package-tui-tabs/`（`package-tui-*` 落在嵌套包实例）。根 specs-landed 只扫 `llmanspec/specs/`，且 `@req` 跨根必须唯一，故不在根树再抄一份；本 change 标 `needs_specs_change: false` 以过 apply 门。

## Open Questions

1. ~~首切片是否只做包 Tab？~~ **只做刀 A**（2026-10-11）。刀 B 无 runtime。
2. ~~Down 门控：有孩子且 editor 空 vs 另开和弦？~~ **推迟到刀 B**。
3. ~~产品顶栏多会话 tab？~~ **否**。默认仍一次一会话 + resume 槽。
4. ~~包层 Tabs 拥有 page 还是只画条？~~ **只画条**（2026-10-11）。对齐 zellij tab-bar / opencode session-tabs。
5. ~~溢出：窗口 + 两侧计数 vs 全画截断 vs 本票先不做？~~ **窗口 + 两侧计数**（2026-10-11）。装不下就少画完整 title，两侧 `← +N` / `+N →`，当前 title 必须完整可见。
6. ~~用户按键时，谁改标题条下标？~~ **登记 id、默认不绑物理键**（2026-10-11）。`tui.tabs.prev` / `tui.tabs.next` 默认键列表为空，与 `tui.select.pageUp` 相同。数字跳转不进默认表。
7. ~~本票要不要支持标题左右分列、整条隐藏？~~ **不要**（2026-10-11）。以后另开 change。
8. ~~超宽藏掉若干标题后，列区间怎么对回原来的标题下标？~~ **每段带原下标**（2026-10-11）：`(标题下标, 起始列, 结束列)`，只含画出来的标题。
9. ~~两侧溢出标记要不要也给列区间？~~ **要**（2026-10-11）。按 zellij：每侧标记有列区间，并带该侧第一个隐藏标题的原下标。点击走 Q10。
10. ~~本票标题条要不要自己处理点击？~~ **要**（2026-10-11）。左键按下：列落在标题上则选该标题；落在溢出标记上则选该侧第一个隐藏标题。不登记命中区。
11. ~~标题条上滚轮要不要切相邻标题？~~ **要**（2026-10-11）。向上滚选下一项，向下滚选上一项，循环。与 zellij tab-bar 相同。
12. ~~调用方怎么把屏上指针交给标题条？~~ **库做命中与投递**（2026-10-11）。调用方只报标题条画在屏上哪一行。`set_screen_origin` + `offer_pointer`；`TUI::dispatch_event` 在焦点路由前投给根 child。区间、`set_selected`、条内列的 `handle_input` 仍可用。不改 `Container` 对键盘的 fan-out（pte01 / r1614）。

## Further Notes

- [research/tabs-multi-agent-peers.md](./research/tabs-multi-agent-peers.md)
- [research/opencode2-zellij-tabs.md](./research/opencode2-zellij-tabs.md)（`../opencode` `beta`、`../zellij` `main`）
- `docs/roadmaps/Sub-Agent编排.md` · delayed c2325
