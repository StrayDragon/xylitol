# Research: Zellij 选择滚动 / 选择复制 / 输入区特别处理（Mode B 对照）

> 日期：2026-08-11
> Change：`c2070-add-package-tui-dual-interaction-modes`
> 一手：`/home/l8ng/Projects/__straydragon__/zellij`（zellij-server / zellij-utils / default-plugins）
> 目的：为 xylitol Mode B（应用内选区 + app scroll + 底部 Editor）提供可借鉴实现事实，非照搬清单。

## 0. 结论摘要

| 主题 | Zellij 事实 |
|---|---|
| 选区模型 | `Selection{start,end,active}`；半开区间；跨行；负 line = scrollback |
| 鼠标状态机 | `selecting_with_mouse_in_pane` 钉住拖选 pane；Press→Start / Motion→Update / Release→End |
| 越界续选 | 相对坐标越出内容区则节流滚 1 行（10ms）；`active` 时只平移锚点 `start` |
| 松手复制 | `copy_on_select` 默认 true；Release 调 `write_selection_to_clipboard` |
| 剪贴板 | 有 `copy_command` → 本地命令；否则 OSC52 |
| 不可选 / 冲突 | frame 优先 resize/move；子程序 mouse tracking 优先于应用选区；chrome 插件 `set_selectable(false)` |

---

## 1. Selection 数据模型

源：`zellij-server/src/panes/selection.rs` → `Selection`

- 字段：`start` / `end`（`Position`）、`active: bool`（拖选中）、`last_added_word_position` / `last_added_line_index`（双击词 / 三击行扩展）。
- 空选：`start == end`（文件头注释）；`contains` 对同列半开：`col < end.column`。
- API：`start()` 置 `active=true` 且起止同点；`to()` 只改 `end`；`end()` / `finalize()` 置 `active=false`。
- 跨行：`contains` / `line_indices` / `sorted()` 用起止序无关比较。
- **滚动时语义**：`move_up` / `move_down` — **仅当 `!active` 才同时平移 `end`**；拖选中只移 `start`（锚点随视口平移，活动端留给鼠标）。

`Position`（`zellij-utils/src/position.rs`）：`line: isize`（可负）、`column: usize`。`relative_to(content_y, content_x)` 把屏坐标减成 pane 内容相对坐标。

文本抽取：`Grid::get_selected_text`（`panes/grid.rs`）按 sorted 区间扫 `lines_above`（负 index）/ `viewport` / `lines_below`，行尾 `trim_end` 后 `\n` 拼接。

---

## 2. Mouse 状态机

源：`zellij-server/src/tab/mouse_handler.rs`

入口：`MouseHandler::handle_mouse_event` → `gather_mouse_event_context` → `determine_mouse_action` → `execute_mouse_action`。

`MouseAction` 选区相关：`StartSelection { pane_id, position }` / `UpdateSelection { position }` / `EndSelection { position }`。

**左键拖选主路径**（`determine_mouse_action`）：

1. 若已在 `pane_being_resized` / `selecting_with_mouse` / `pane_being_moved` → 优先续该手势（选区：左键 Motion→Update，Release→End）。
2. 普通左键 Press 且点在**已聚焦** pane、且不在 frame：
   - `terminal_wants_mouse`（子程序开了 mouse tracking）→ `SendToTerminal`，**不**开应用选区；
   - 否则 → `StartSelection`。
3. 点在非焦点 pane → `FocusPane`（或 `FocusPaneAndClickThrough`），一般不立刻进入拖选钉住（除非 click-through 且子程序不要鼠）。

**执行**：

- `StartSelection`：`pane.start_selection`；若 `supports_mouse_selection()` 则 `tab.selecting_with_mouse_in_pane = Some(pane_id)`。若已有选中文本（双/三击立刻成选）→ `MouseEffect::leave_clipboard_message()`。
- `UpdateSelection`：只对钉住的 pane 调 `update_selection`（鼠标可飞出该 pane 几何，仍续选）。
- `EndSelection`：见 §4。

`ClickedPaneDetails.terminal_wants_mouse`：仅当 pane 已是 active 且 `mouse_left_click(..., false).is_some()`（`Grid::mouse_left_click_signal`，要求 `mouse_tracking != Off`）。

---

## 3. 选择滚动（越界续选）

源：`TerminalPane::update_selection`（`panes/terminal_pane.rs`）+ `SELECTION_SCROLL_INTERVAL_MS = 10` + `Grid::scroll_up_one_line` / `scroll_down_one_line`。

相对内容坐标 `to`：

| 条件 | 动作 |
|---|---|
| `to.line < 0` 且距上次滚 ≥ 10ms | `scroll_up_one_line()`（从 `lines_above` 拉入视口），刷新 `selection_scrolled_at` |
| `to.line as usize >= grid.height` 且节流过 | `scroll_down_one_line()` |
| 在 `[0, height)` | `grid.update_selection(to)` |

注意：源码变量名 `cursor_at_the_bottom` 对应 **`line < 0`（内容区上方）**；`cursor_at_the_top` 对应 **越出下沿**——命名与视觉顶/底相反，以坐标为准。

滚动与选区坐标：

- `scroll_up_one_line` → `selection.move_down(1)`（`grid.rs`）
- `scroll_down_one_line` → `selection.move_up(1)`
- 拖选 `active==true` 时只移锚点，活动端保持在边界附近，直到鼠标回到内容区再 `to()`。

越界时**不**调用 `grid.update_selection`，纯靠「滚一行 + 平移锚点」续选。注释 TODO：可按越出距离加大滚速（尚未做）。

插件 pane：`PluginPane::update_selection` 在 `supports_mouse_selection` 时直接 `grid.update_selection`，**无**上述边界自动滚逻辑。

---

## 4. 选择复制

| 符号 | 位置 | 行为 |
|---|---|---|
| `Tab.copy_on_select` | `tab/mod.rs` 构造自 `copy_options.copy_on_select` | 默认来自配置；`options.copy_on_select.unwrap_or(true)`（`screen.rs`） |
| `execute_end_selection` | `mouse_handler.rs` | Release：相对坐标 clamp 到内容宽高；若子程序要 release 信号则转发 VTE；否则 `end_selection`；若 `copy_on_select` 且 `get_selected_text` 有值 → `write_selection_to_clipboard` + `leave_clipboard_message`；清 `selecting_with_mouse_in_pane` |
| `write_selection_to_clipboard` | `tab/mod.rs` | 委托 `clipboard_provider.set_content`，成功/失败发插件事件 |
| `ClipboardProvider` | `tab/clipboard.rs` | `Command(CopyCommand)` **或** `Osc52(Clipboard)`（互斥） |
| OSC52 | `ClipboardProvider::set_content` | `\x1b]52;{c\|p};{base64}\x1b\\` 写入 client pre-VTE |
| 本地命令 | 有 `copy_command` 时（`Tab` 构造 / 热更新） | `ClipboardProvider::Command`，**不走** OSC52 |
| `copy_selection` | `tab/mod.rs` | 快捷键路径：active pane 选区 → 同一 `write_selection_to_clipboard` |
| `leave_clipboard_message` | `MouseEffect` / `screen.rs` 处理鼠标结果 | 为 true 时**不**向插件广播 `InputReceived`（避免清掉「已复制」类 UI）；非「是否复制」开关 |

双/三击：`Grid::start_selection` 经 click 检测选词/整行；`end_selection` 对双三击走 `finalize()` 而不把 end 改成松手坐标。

---

## 5. 输入框 / 不可选 / 特别处理

Zellij **没有** xylitol 式「底部 Editor 组件」；等价冲突面如下。

**几何 / chrome**

- 选区坐标相对 **content**（`Pane::relative_position` → `get_content_y/x`），frame 不在内容网格内。
- 点在 frame：`position_is_on_frame` → 浮动/钉住则 `StartMovingFloatingPane`，平铺则 `StartResize`，**不** StartSelection。
- UI chrome 多为插件：`tab-bar` / `status-bar` / `compact-bar` / `multiple-select` 在 `load` 调 `set_selectable(false)`（`zellij-tile::set_selectable`）。

**selectable vs 选区**

- `Pane::selectable()`：能否被 focus / hover-focus（`FocusOnHover` 跳过 unselectable）。
- `focus_pane_at(..., search_selectable=true)` 只聚焦可选 pane。
- 点到 unselectable：`execute_focus_pane` 仍可能 `unselectable_pane_at_position` → `start_selection`（对插件常变成 Mouse 事件，见下）。
- 钉住的 unselectable floating：普通左键 → `NoAction`（`pinned_unselectable`）。

**插件选区开关**

- `PluginPane` 默认 `supports_mouse_selection: false`（`plugin_pane.rs` 构造）。
- false：`start/update/end_selection` 转发 `Event::Mouse::{LeftClick,Hold,Release}` 给插件，**不**钉 `selecting_with_mouse_in_pane`（因 `supports_mouse_selection()` 为 false）。
- true：走与终端类似的 per-client `Grid` 选区；`set_mouse_selection_support(false)` 会 `reset_selection`。
- trait 默认 `supports_mouse_selection() -> true`（`tab/mod.rs` `Pane`）；终端走默认。

**与「输入焦点」谁赢**

1. **子程序 mouse tracking**（vim/tmux 等）> 应用选区：active + `terminal_emulator_wants_mouse` → 全程 `SendToTerminal`。
2. **拖选手势进行中** > 新 hover/focus：`selecting_with_mouse` 短路其它动作。
3. **frame 手势** > 内容选区。
4. 键盘焦点仍在 active pane；选区是鼠标手势状态，不抢键盘路由——但 tracking 开启时鼠标字节进 PTY，等于把输入面交给子程序。

浮动层：floating 可见时 hit-test 优先 floating；选区钉在开始拖选的那个 pane id。

---

## 6. 对 xylitol Mode B 的可借鉴 / 不可照搬

**可借鉴**

- 钉住「拖选所有者」id，Motion/Release 不跟 hit-test 漂移。
- 越界用**相对内容坐标**越界检测 + **节流滚一行** + 锚点随 scroll 平移（`active` 只移 start）。
- 松手：clamp → finalize → 可选自动复制；复制后端 OSC52 / 本地命令可配置、默认开。
- 内容区 vs chrome：选区只吃 transcript/viewport，frame/status/底部 Editor 单独 hit 规则。
- 双击词 / 三击行作 SHOULD，与拖选同一 `Selection`。
- 「应用要鼠」vs「子区域自己要鼠」的 oneof（Zellij：tracking；Mode B：折叠 hit / Editor 焦点）。

**不可照搬**

- 多 pane / floating / plugin `selectable` 模型 ≠ 单 surface + 底部 Editor。
- 负 line scrollback 坐标绑在 Zellij `Grid`；Mode B 宜用 transcript 绝对行号 + viewport 窗口。
- `leave_clipboard_message` 是 Zellij 插件 toast 协议，不必原样。
- 插件默认关 mouse selection、点进插件变 Click 事件——Editor 应明确：落在输入区则**不**开 transcript 拖选（或显式取消选区），而不是「unselectable 仍 start_selection」。
- 10ms 滚 1 行可作起点；Mode B 可按越出距离加速（Zellij TODO）。

---

## 7. 对比表：Zellij vs Pi AltScreen

> Pi 列已由主会话按 [`pi-altscreen-selection-scroll-copy-input.md`](./pi-altscreen-selection-scroll-copy-input.md) 合并（2026-08-11）。

| 维度 | Zellij（本文一手） | Pi `TuiAltScreen` |
|---|---|---|
| 选区归属 | 复用器应用层（per-pane `Grid`） | 应用层（`selectionAnchor` / `selectionFocus`） |
| 越界续选 | 相对坐标越界 + **10ms**/行 + `active` 时只平移锚点 `start` | 贴 ScrollView 顶/底 → `setInterval(**50ms**)` + `ScrollView.scrollBy`；固定 drag pointer 续 focus |
| 松手复制 | `copy_on_select` 默认 true → 有 `copy_command` 走本地，否则 **OSC52** | `copySelectionToClipboard`：**仅 OSC52**；无 wl-copy/pbcopy 回退；不清选区，flash「Copied!」 |
| 与输入区冲突 | frame / 子程序 mouse tracking / `set_selectable(false)` | Editor 在 ScrollView **外** dock；高亮裁到 ScrollView box；鼠标一律 consume，**无** click-to-focus |
| 坐标 | viewport 相对，负行 = scrollback | ScrollView 视口（非终端 scrollback） |
| 产品形态 | 多 pane 终端复用器 | 单 agent TUI fullscreen |

---

## 关键路径速查

- `zellij-server/src/panes/selection.rs` — `Selection`
- `zellij-server/src/tab/mouse_handler.rs` — `MouseAction::*Selection*`, `execute_end_selection`
- `zellij-server/src/panes/terminal_pane.rs` — `update_selection`, `SELECTION_SCROLL_INTERVAL_MS`
- `zellij-server/src/panes/grid.rs` — `start/update/end_selection`, `scroll_*_one_line`, `get_selected_text`
- `zellij-server/src/tab/clipboard.rs` — `ClipboardProvider`
- `zellij-server/src/tab/mod.rs` — `write_selection_to_clipboard`, `Pane::relative_position`
- `zellij-server/src/panes/plugin_pane.rs` — `supports_mouse_selection`
- `default-plugins/{tab-bar,status-bar,compact-bar}/src/main.rs` — `set_selectable(false)`
