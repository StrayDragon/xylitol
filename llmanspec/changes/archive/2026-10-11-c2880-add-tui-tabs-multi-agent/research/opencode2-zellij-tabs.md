# opencode beta / zellij Tab 一手对照

读到的树：

- `../opencode` 当前分支 `beta` HEAD `e5ecb571`（用户说的 opencode2）
- `../zellij` 当前分支 `main` @ `98cb434d`

没有改这两个仓库。

## 两边都不是「组件拥有 page 子树」

| 层 | 谁拥有内容 | 标题条做什么 |
|---|---|---|
| zellij server `Tab` | 一个 tab 持有 tiled/floating panes（`zellij-server/src/tab/mod.rs:1-2`） | 与画条无关 |
| zellij `tab-bar` 插件 | 只存 `Vec<TabInfo>` + `active_tab_idx`（`default-plugins/tab-bar/src/main.rs:29-36`） | 画一行；点击/滚轮发 `switch_tab_to` |
| opencode2 桌面 `TabsProvider` | `Tab = session \| draft`，`select` = `navigate(href)`（`packages/app/src/shell/tabs/tabs.tsx:43-47,174-178,468`） | 标题条不挂 page 子组件 |
| opencode2 桌面 in-session tabs | `SessionTabs = { active?, all: string[] }`，id 是 `"review"` / `"context"` / `"open-file"` / `"browser:…"`（`packages/app/src/shell/state/session-tabs.ts:7-15`） | 第二套：会话内面板，不是顶栏 |
| opencode TUI `SessionTab` | `{ sessionID, title? }`（`packages/tui/src/context/session-tabs-model.ts:1-4`） | 画条 + 切 **root** session；`select` canonicalize 成 root（`session-tabs.tsx:371-374`）；transcript 是 `app.tsx` 里条的兄弟 `SessionFrame`，不是 children |

Q4 已锁 chrome（2026-10-11）：不持有 page。notebook 废弃。

## 溢出：窗口，不是把每个 title 截成均宽

zellij `populate_tabs_in_tab_line`（`default-plugins/tab-bar/src/line.rs:15-107`）：

- 先放 active，再左右轮流加，加不下就停
- 两侧用 `← +N` / `+N →` 表示藏着的 tab
- 单条标题过长也不截断（只追加 ` (FULLSCREEN)` / ` (SYNC)` / ` [!]`，`tab.rs:106-113`）
- 点击 overflow 块也带 `tab_index`（`line.rs:141,173` + `tab.rs:122-135`）
- 条是 layout 里 height=1、`set_selectable(false)` 的插件 pane（`tab-bar/src/main.rs:48,119`；`layouts/default.kdl:1-9`），不是 `Tab` 对象的字段

opencode TUI `adaptiveSessionTabLayout`（`session-tabs-model.ts:175-257`）：

- 算出可见窗口 `start..start+count`，保证 active 在窗内
- 两侧宽度用 `sessionTabOverflowWidth`（`‹12 ` / ` 12›`，`session-tabs-model.ts:36-37`）
- 可见项有 `MIN=8 / 默认=22 / MAX=32`；挤的时候 active 尽量拿满 22，其余均分
- 水平条 `overflow-x-auto` + 左右 fade（桌面 `tab-strip.tsx:285-404`）

现设计 D8「每个 title 至少 1 列、先保 selected、其余均分截断」是第三种算法。两边都是「少画几个完整 title + 溢出标记」，不是「全画但全截」。

## 键：切 tab ≠ 裸 Left/Right

opencode TUI 默认（`packages/tui/src/config/keybind.ts:112-156`）：

| 动作 | 默认和弦 |
|---|---|
| `session.tab.next` / `previous` | `ctrl+tab` / `ctrl+shift+tab`（兼 `alt+down` / `alt+up`） |
| `session.tab.select.1`–`10` | `<leader>N` 兼 `ctrl+N`（不是裸 `1`–`9`） |
| `session.child.first` | 裸 `down`（子 agent picker） |
| `session.child.next` / `previous` | 裸 `right` / `left`——**无 handler**（全仓除 `keybind.ts:134-135` 无接线；v1 映射也没有） |
| `session.parent` | 裸 `up` |

桌面顶栏另注册 `tab.next` = `mod+option+ArrowRight,ctrl+tab`（`tab-strip.tsx:251-267`）；数字是 `mod+1`–`mod+9`（`tab-strip.tsx:409-425`）。

zellij 默认（`zellij-utils/assets/config/default.kdl:51-71,184-194`）：

- 先 `Ctrl t` 进 `tab` mode
- 在 mode 里：`Left`/`Right`/`h`/`l`/`j`/`k` 切 tab；`1`–`9` = `GoToTab` 后回 Normal
- Normal 下裸方向键不切 tab
- `Alt n` 是 `NewPane`，不是 NewTab
- `MoveFocusOrTab`（`zellij-server/src/route.rs:384-412`，`screen.rs:3910-3979`）：Left/Right 先在当前 tab 里移 pane 焦点，失败才切 tab。这是 multiplexer，不是 Tabs widget 该绑的默认键

Composer 打开时 Left/Right 切 composer 内页（subagents / shell / terminals），与 session tab 无关（`routes/session/composer/index.tsx:98-104`）。editor 层吃 `input.move.left/right`。Down 同时绑 `prompt.history.next` 和 `session.child.first`；谁赢无专门测试。

Q6 已锁：登记 `tui.tabs.prev` / `next`，默认不绑物理键。

## 鼠标（两边都有，现票刻意不做）

- zellij：`LeftClick(_, col)` 走 `get_tab_to_focus`（累计 `LinePart.len`）；滚轮切邻 tab（`tab-bar/src/main.rs:92-104`，`tab.rs:122-148`）
- opencode 桌面：mousedown 切 tab、中键关、拖排序（`tab-nav.tsx:201-238`，`tab-strip.tsx:292-334`）
- opencode TUI：有 `session-tabs-mouse.test.tsx`

`title_col_ranges` 对以后点选是对的，形状接近 zellij 的 `LinePart { len, tab_index }`。本票仍可不接鼠标。

## 对 xylitol 的含义（未改合约）

可抄：

- 标题条是 chrome：titles + selected + 列几何。内容由 host / server 换，不由条持有
- 溢出用窗口 + 两侧计数，保证 active 完整可见
- 切 tab 的键和 editor / child 分开；数字跳转用修饰键或 mode，不要默认抢裸 `1`–`9`

不要抄进本票：

- unread / spinner / marquee / rename / 关 tab 栈（opencode TUI 产品层）
- 多客户端光标、FULLSCREEN/SYNC 后缀（zellij）
- 桌面拖排序、中键关、preview popover
- Down 开 picker（刀 B，无 runtime）
- 把 `session.child.next/previous` 当已实现导航（opencode TUI 是死键）
- 把子 agent 做成顶栏 tab 页（opencode canonicalize 到 root；picker 是另一套 Composer）

已锁到 Q12：库做 `offer_pointer` / origin；`dispatch_event` 焦点前投递。
